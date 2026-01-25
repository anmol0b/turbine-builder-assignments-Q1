import { Commitment, Connection, Keypair, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js"
import wallet from "./wallet/turbin3-wallet.json"
import { getOrCreateAssociatedTokenAccount, transfer } from "@solana/spl-token";

// We're going to import our keypair from the wallet file
const keypair = Keypair.fromSecretKey(new Uint8Array(wallet));

//Create a Solana devnet connection
const commitment: Commitment = "confirmed";
const connection = new Connection("https://api.devnet.solana.com", commitment);

// Mint address
const mint = new PublicKey("855GVDkGkearvQmaiUrZEwSuCzpPAtnV2eadVuvy3XXh");

// Recipient address
const to = new PublicKey("CSMFTP3DuMqWZLRQBQwxqNHoDxdpstTaub1LkAhazq1T");

(async () => {
    try {
        // Get the token account of the fromWallet address, and if it does not exist, create it
        const from_ata = await getOrCreateAssociatedTokenAccount(
            connection,
            keypair,
            mint,
            keypair.publicKey,
        );
        // Get the token account of the toWallet address, and if it does not exist, create it
        const to_ata = await getOrCreateAssociatedTokenAccount(
            connection,
            keypair,
            mint,
            to
        );
        // Transfer the new token to the "toTokenAccount" we just created
        const transfer_sig = await transfer(
            connection,
            keypair,
            from_ata.address,
            to_ata.address,
            keypair,
            100_000000n, // 1e6 = 1 token 
        );

        console.log(transfer_sig);
    } catch(e) {
        console.error(`Oops, something went wrong: ${e}`)
    }
})();