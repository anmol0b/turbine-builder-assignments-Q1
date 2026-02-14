import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AnchorDice2024 } from "../target/types/anchor_dice_2024";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
  Ed25519Program,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { assert, expect } from "chai";

describe("anchor-dice-game-q4-25", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.AnchorDice2024 as Program<AnchorDice2024>;
  const connection = provider.connection;

  const house = Keypair.generate();
  const player = Keypair.generate();

  const instructionSysvar = new PublicKey(
    "Sysvar1nstructions1111111111111111111111111",
  );

  let vault: PublicKey;

  const HOUSE_FEE_BPS = 150;
  const BPS = 10_000;

  const confirm = async (sig: string) => {
    const latest = await connection.getLatestBlockhash();
    await connection.confirmTransaction(
      { signature: sig, ...latest },
      "confirmed",
    );
  };

  before(async () => {
    const fundTx = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: provider.wallet.publicKey,
        toPubkey: house.publicKey,
        lamports: 2 * LAMPORTS_PER_SOL,
      }),
      SystemProgram.transfer({
        fromPubkey: provider.wallet.publicKey,
        toPubkey: player.publicKey,
        lamports: 2 * LAMPORTS_PER_SOL,
      }),
    );

    const payer = (provider.wallet as any).payer;
    
    const sig = await sendAndConfirmTransaction(connection, fundTx, [payer]);


    await confirm(sig);

    [vault] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), house.publicKey.toBytes()],
      program.programId,
    );
  });

  it("initialize: loads vault with funds", async () => {
    const amount = new anchor.BN(2 * LAMPORTS_PER_SOL);

    const sig = await program.methods
      .initialize(amount)
      .accountsStrict({
        house: house.publicKey,
        vault,
        systemProgram: SystemProgram.programId,
      })
      .signers([house])
      .rpc();

    await confirm(sig);

    const vaultBalance = await connection.getBalance(vault);
    expect(vaultBalance).to.eq(amount.toNumber());
  });

  it("placeBet: creates bet + moves funds to vault", async () => {
    const seed = new anchor.BN(1);
    const roll = 50;
    const amount = new anchor.BN(LAMPORTS_PER_SOL / 10);

    const [betPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("bet"), vault.toBytes(), seed.toArrayLike(Buffer, "le", 16)],
      program.programId,
    );

    const vaultBefore = await connection.getBalance(vault);

    const sig = await program.methods
      .placeBet(seed, roll, amount)
      .accountsStrict({
        player: player.publicKey,
        house: house.publicKey,
        vault,
        bet: betPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([player])
      .rpc();

    await confirm(sig);

    const bet = await program.account.bet.fetch(betPda);

    expect(bet.player.toBase58()).to.eq(player.publicKey.toBase58());
    expect(bet.seed.toString()).to.eq(seed.toString());
    expect(bet.roll).to.eq(roll);
    expect(bet.amount.toString()).to.eq(amount.toString());

    const vaultAfter = await connection.getBalance(vault);
    expect(vaultAfter - vaultBefore).to.eq(amount.toNumber());
  });

  it("refundBet: rejects before timeout (unhappy path)", async () => {
    const seed = new anchor.BN(99);
    const roll = 50;
    const amount = new anchor.BN(LAMPORTS_PER_SOL / 10);

    const [betPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("bet"), vault.toBytes(), seed.toArrayLike(Buffer, "le", 16)],
      program.programId,
    );

    const sig = await program.methods
      .placeBet(seed, roll, amount)
      .accountsStrict({
        player: player.publicKey,
        house: house.publicKey,
        vault,
        bet: betPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([player])
      .rpc();

    await confirm(sig);

    try {
      await program.methods
        .refundBet()
        .accountsStrict({
          player: player.publicKey,
          house: house.publicKey,
          vault,
          bet: betPda,
          systemProgram: SystemProgram.programId,
        })
        .signers([player])
        .rpc();

      assert.fail("refundBet should fail before timeout");
    } catch (err) {
      expect(err).to.exist;
    }
  });

  it("resolveBet: happy path (player wins)", async () => {
    const seed = new anchor.BN(100);
    const roll = 100;
    const amount = new anchor.BN(LAMPORTS_PER_SOL / 10);

    const [betPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("bet"), vault.toBytes(), seed.toArrayLike(Buffer, "le", 16)],
      program.programId,
    );

    const sig1 = await program.methods
      .placeBet(seed, roll, amount)
      .accountsStrict({
        player: player.publicKey,
        house: house.publicKey,
        vault,
        bet: betPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([player])
      .rpc();

    await confirm(sig1);

    const betInfo = await connection.getAccountInfo(betPda);
    if (!betInfo) throw new Error("bet account missing");

    const betRent = betInfo.lamports;

    const ed25519Ix = Ed25519Program.createInstructionWithPrivateKey({
      privateKey: player.secretKey,
      message: betInfo.data.subarray(8),
    });

    const signature = ed25519Ix.data.subarray(
      ed25519Ix.data.length - 64,
      ed25519Ix.data.length,
    );

    const resolveIx = await program.methods
      .resolveBet(signature)
      .accountsStrict({
        player: player.publicKey,
        house: house.publicKey,
        vault,
        bet: betPda,
        instructions: instructionSysvar,
        systemProgram: SystemProgram.programId,
      })
      .instruction();

    const vaultBefore = await connection.getBalance(vault);
    const playerBefore = await connection.getBalance(player.publicKey);

    const tx = new Transaction().add(ed25519Ix).add(resolveIx);
    const sig2 = await provider.sendAndConfirm(tx, [house]);


    await confirm(sig2);

    const closed = await connection.getAccountInfo(betPda);
    expect(closed).to.eq(null);

    const vaultAfter = await connection.getBalance(vault);
    const playerAfter = await connection.getBalance(player.publicKey);

    const expectedPayout = Math.floor(
      (amount.toNumber() * (BPS - HOUSE_FEE_BPS)) / BPS,
    );

    expect(vaultBefore - vaultAfter).to.eq(expectedPayout);
    expect(playerAfter - playerBefore).to.eq(betRent + expectedPayout);
  });

  it("resolveBet: unhappy path (player loses)", async () => {
    const seed = new anchor.BN(101);
    const roll = 1;
    const amount = new anchor.BN(LAMPORTS_PER_SOL / 10);

    const [betPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("bet"), vault.toBytes(), seed.toArrayLike(Buffer, "le", 16)],
      program.programId,
    );

    const sig1 = await program.methods
      .placeBet(seed, roll, amount)
      .accountsStrict({
        player: player.publicKey,
        house: house.publicKey,
        vault,
        bet: betPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([player])
      .rpc();

    await confirm(sig1);

    const betInfo = await connection.getAccountInfo(betPda);
    if (!betInfo) throw new Error("bet account missing");

    const betRent = betInfo.lamports;

    const ed25519Ix = Ed25519Program.createInstructionWithPrivateKey({
      privateKey: player.secretKey,
      message: betInfo.data.subarray(8),
    });

    const signature = ed25519Ix.data.subarray(
      ed25519Ix.data.length - 64,
      ed25519Ix.data.length,
    );

    const resolveIx = await program.methods
      .resolveBet(signature)
      .accountsStrict({
        player: player.publicKey,
        house: house.publicKey,
        vault,
        bet: betPda,
        instructions: instructionSysvar,
        systemProgram: SystemProgram.programId,
      })
      .instruction();

    const vaultBefore = await connection.getBalance(vault);
    const playerBefore = await connection.getBalance(player.publicKey);

    const tx = new Transaction().add(ed25519Ix).add(resolveIx);
    const sig2 = await sendAndConfirmTransaction(connection, tx, [house], {
      commitment: "confirmed",
    });

    await confirm(sig2);

    const closed = await connection.getAccountInfo(betPda);
    expect(closed).to.eq(null);

    const vaultAfter = await connection.getBalance(vault);
    const playerAfter = await connection.getBalance(player.publicKey);

    expect(vaultBefore - vaultAfter).to.eq(0);
    expect(playerAfter - playerBefore).to.eq(betRent);
  });
});
