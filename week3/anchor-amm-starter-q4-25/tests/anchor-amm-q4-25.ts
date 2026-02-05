import * as anchor from "@coral-xyz/anchor";
import { BN, Program, web3 } from "@coral-xyz/anchor";
import { AnchorAmmQ425 } from "../target/types/anchor_amm_q4_25";

import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  getAccount,
  getAssociatedTokenAddress,
  getMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";
import { assert } from "chai";

const { Keypair, PublicKey } = web3;

describe("anchor-amm-q4-25", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const connection = provider.connection;
  const program = anchor.workspace.anchorAmmQ425 as Program<AnchorAmmQ425>;

  const payer = provider.wallet.payer;

  const SEED = new BN(1);
  const FEE = 500;
  const DECIMALS = 6;

  const authority = Keypair.generate();
  const mintX = Keypair.generate();
  const mintY = Keypair.generate();
  const user = Keypair.generate();

  let configPda: web3.PublicKey;
  let lpMintPda: web3.PublicKey;
  let configBump: number;
  let lpBump: number;

  let vaultX: web3.PublicKey;
  let vaultY: web3.PublicKey;

  let userX: web3.PublicKey;
  let userY: web3.PublicKey;
  let userLp: web3.PublicKey;

  const base = (n: number) => new BN(n * 10 ** DECIMALS);
  const bi = (n: BN) => BigInt(n.toString());

  const expectFail = async (fn: () => Promise<any>, msg?: string) => {
    let threw = false;
    try {
      await fn();
    } catch (e) {
      threw = true;
    }
    assert(threw, msg ?? "expected tx to fail");
  };

  before(async () => {
    await provider.connection.requestAirdrop(payer.publicKey, 2_000_000_000);
    await provider.connection.requestAirdrop(
      authority.publicKey,
      2_000_000_000,
    );
    await provider.connection.requestAirdrop(user.publicKey, 2_000_000_000);

    await new Promise((r) => setTimeout(r, 800));

    [configPda, configBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), new Uint8Array(SEED.toArray("le", 8))],
      program.programId,
    );

    [lpMintPda, lpBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), configPda.toBytes()],
      program.programId,
    );

    await createMint(
      connection,
      payer,
      authority.publicKey,
      null,
      DECIMALS,
      mintX,
    );

    await createMint(
      connection,
      payer,
      authority.publicKey,
      null,
      DECIMALS,
      mintY,
    );

    vaultX = await getAssociatedTokenAddress(mintX.publicKey, configPda, true);
    vaultY = await getAssociatedTokenAddress(mintY.publicKey, configPda, true);

    userX = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        payer,
        mintX.publicKey,
        user.publicKey,
      )
    ).address;

    userY = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        payer,
        mintY.publicKey,
        user.publicKey,
      )
    ).address;

    userLp = await getAssociatedTokenAddress(lpMintPda, user.publicKey);

    const mintAmount = BigInt(1000) * BigInt(10 ** DECIMALS);

    await mintTo(connection, payer, mintX.publicKey, userX, authority, mintAmount);
    await mintTo(connection, payer, mintY.publicKey, userY, authority, mintAmount);
  });

  it("initialize works", async () => {
    await program.methods
      .initialize(SEED, FEE, authority.publicKey)
      .accountsStrict({
        initializer: payer.publicKey,
        mintX: mintX.publicKey,
        mintY: mintY.publicKey,
        mintLp: lpMintPda,
        vaultX,
        vaultY,
        config: configPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SYSTEM_PROGRAM_ID,
      })
      .signers([payer])
      .rpc();

    const cfg = await program.account.config.fetch(configPda);

    assert(cfg.seed.eq(SEED));
    assert(cfg.fee === FEE);
    assert(cfg.locked === false);

    assert(cfg.mintX.equals(mintX.publicKey));
    assert(cfg.mintY.equals(mintY.publicKey));
    assert(cfg.authority.equals(authority.publicKey));

    assert(cfg.configBump === configBump);
    assert(cfg.lpBump === lpBump);
  });

  it("initialize should not run twice", async () => {
    await expectFail(async () => {
      await program.methods
        .initialize(SEED, FEE, authority.publicKey)
        .accountsStrict({
          initializer: payer.publicKey,
          mintX: mintX.publicKey,
          mintY: mintY.publicKey,
          mintLp: lpMintPda,
          vaultX,
          vaultY,
          config: configPda,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SYSTEM_PROGRAM_ID,
        })
        .signers([payer])
        .rpc();
    });
  });

  it("deposit mints LP and moves tokens", async () => {
    const lpBefore = await getMint(connection, lpMintPda);

    const amount = base(200);
    const maxX = base(100);
    const maxY = base(100);

    await program.methods
      .deposit(amount, maxX, maxY)
      .accountsStrict({
        user: user.publicKey,
        mintX: mintX.publicKey,
        mintY: mintY.publicKey,
        config: configPda,
        mintLp: lpMintPda,
        vaultX,
        vaultY,
        userX,
        userY,
        userLp,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SYSTEM_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

    const vaultXAcc = await getAccount(connection, vaultX);
    const vaultYAcc = await getAccount(connection, vaultY);
    const userLpAcc = await getAccount(connection, userLp);

    const lpAfter = await getMint(connection, lpMintPda);

    assert(vaultXAcc.amount === bi(maxX));
    assert(vaultYAcc.amount === bi(maxY));
    assert(userLpAcc.amount === bi(amount));

    assert(lpAfter.supply > lpBefore.supply);
  });

  it("deposit fails if max is too small", async () => {
    await expectFail(async () => {
      await program.methods
        .deposit(base(50), new BN(1), new BN(1))
        .accountsStrict({
          user: user.publicKey,
          mintX: mintX.publicKey,
          mintY: mintY.publicKey,
          config: configPda,
          mintLp: lpMintPda,
          vaultX,
          vaultY,
          userX,
          userY,
          userLp,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SYSTEM_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        })
        .signers([user])
        .rpc();
    }, "deposit should fail with slippage");
  });

  it("withdraw burns LP and returns x/y", async () => {
    const userXBefore = await getAccount(connection, userX);
    const userYBefore = await getAccount(connection, userY);
    const lpBefore = await getAccount(connection, userLp);

    const vaultXBefore = await getAccount(connection, vaultX);
    const vaultYBefore = await getAccount(connection, vaultY);

    const amount = base(100);

    await program.methods
      .withdraw(amount, new BN(0), new BN(0))
      .accountsStrict({
        user: user.publicKey,
        mintX: mintX.publicKey,
        mintY: mintY.publicKey,
        config: configPda,
        mintLp: lpMintPda,
        vaultX,
        vaultY,
        userX,
        userY,
        userLp,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

    const userXAfter = await getAccount(connection, userX);
    const userYAfter = await getAccount(connection, userY);
    const lpAfter = await getAccount(connection, userLp);

    const vaultXAfter = await getAccount(connection, vaultX);
    const vaultYAfter = await getAccount(connection, vaultY);

    assert(lpAfter.amount === lpBefore.amount - bi(amount));

    assert(userXAfter.amount > userXBefore.amount);
    assert(userYAfter.amount > userYBefore.amount);

    assert(vaultXAfter.amount < vaultXBefore.amount);
    assert(vaultYAfter.amount < vaultYBefore.amount);
  });

  it("withdraw fails if amount > lp balance", async () => {
    const lpAcc = await getAccount(connection, userLp);
    const tooMuch = new BN((lpAcc.amount + BigInt(1)).toString());

    await expectFail(async () => {
      await program.methods
        .withdraw(tooMuch, new BN(0), new BN(0))
        .accountsStrict({
          user: user.publicKey,
          mintX: mintX.publicKey,
          mintY: mintY.publicKey,
          config: configPda,
          mintLp: lpMintPda,
          vaultX,
          vaultY,
          userX,
          userY,
          userLp,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        })
        .signers([user])
        .rpc();
    });
  });

  it("swap x -> y works", async () => {
    const xBefore = await getAccount(connection, userX);
    const yBefore = await getAccount(connection, userY);

    await program.methods
      .swap(true, base(10), new BN(0))
      .accountsStrict({
        user: user.publicKey,
        mintX: mintX.publicKey,
        mintY: mintY.publicKey,
        config: configPda,
        mintLp: lpMintPda,
        vaultX,
        vaultY,
        userX,
        userY,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

    const xAfter = await getAccount(connection, userX);
    const yAfter = await getAccount(connection, userY);

    assert(xAfter.amount < xBefore.amount);
    assert(yAfter.amount > yBefore.amount);
  });

  it("swap y -> x works", async () => {
    const xBefore = await getAccount(connection, userX);
    const yBefore = await getAccount(connection, userY);

    await program.methods
      .swap(false, base(10), new BN(0))
      .accountsStrict({
        user: user.publicKey,
        mintX: mintX.publicKey,
        mintY: mintY.publicKey,
        config: configPda,
        mintLp: lpMintPda,
        vaultX,
        vaultY,
        userX,
        userY,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

    const xAfter = await getAccount(connection, userX);
    const yAfter = await getAccount(connection, userY);

    assert(yAfter.amount < yBefore.amount);
    assert(xAfter.amount > xBefore.amount);
  });

  it("swap fails if minOut is insane", async () => {
    await expectFail(async () => {
      await program.methods
        .swap(true, base(1), base(999999))
        .accountsStrict({
          user: user.publicKey,
          mintX: mintX.publicKey,
          mintY: mintY.publicKey,
          config: configPda,
          mintLp: lpMintPda,
          vaultX,
          vaultY,
          userX,
          userY,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        })
        .signers([user])
        .rpc();
    });
  });
});
