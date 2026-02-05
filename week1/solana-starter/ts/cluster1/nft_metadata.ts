import wallet from "./wallet/turbin3-wallet.json"
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults"
import { createGenericFile, createSignerFromKeypair, signerIdentity } from "@metaplex-foundation/umi"
import { irysUploader } from "@metaplex-foundation/umi-uploader-irys"

// Create a devnet connection
const umi = createUmi('https://api.devnet.solana.com');

let keypair = umi.eddsa.createKeypairFromSecretKey(new Uint8Array(wallet));
const signer = createSignerFromKeypair(umi, keypair);

umi.use(irysUploader());
umi.use(signerIdentity(signer));

(async () => {
    try {
        // Follow this JSON structure
        // https://docs.metaplex.com/programs/token-metadata/changelog/v1.0#json-structure

        const image = "https://gateway.irys.xyz/Hcn6jzsEGpscQe5rdykNeM8UXoWdfk1ccNihYUH3PVL"
        const metadata = {
            name: "purplerug",
            symbol: "PR",
            description: "PR Manipulator",
            image,
            attributes: [
                {trait_type: 'rarity', value: 'legendary'}
            ],
            properties: {
                files: [
                    {
                        type: "image/png",
                        uri: image,
                    },
                ]
            },
            creators: []
        };
        const myUri = await umi.uploader.uploadJson(metadata,{
            onProgress: (percent) => {
                console.log(`Metadata uploaded at : ${percent}\n`)
            },
        });
        console.log("Your metadata URI: ", myUri);
    }
    catch(error) {
        console.log("Oops.. Something went wrong", error);
    }
})();