import {
  createAssociatedTokenAccountIdempotentInstruction,
  createTransferCheckedInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { Connection, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";

const USDC_MINT = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

interface PhantomProvider {
  isPhantom?: boolean;
  publicKey: PublicKey | null;
  connect(): Promise<{ publicKey: PublicKey }>;
  signAndSendTransaction(tx: Transaction): Promise<{ signature: string }>;
}

export function getPhantom(): PhantomProvider | null {
  const w = window as unknown as { phantom?: { solana?: PhantomProvider }; solana?: PhantomProvider };
  return w.phantom?.solana ?? (w.solana?.isPhantom ? w.solana : null);
}

export async function connectSolana(): Promise<string> {
  const p = getPhantom();
  if (!p) throw new Error("Phantom wallet not found");
  const { publicKey } = await p.connect();
  return publicKey.toBase58();
}

/** Send SOL (lamports) or USDC (6dp base units) to a 1Click deposit address. Returns the signature. */
export async function sendSolana(
  rpcUrl: string,
  asset: "SOL" | "USDC",
  depositAddress: string,
  amountBase: bigint,
): Promise<string> {
  const p = getPhantom();
  if (!p?.publicKey) throw new Error("Phantom not connected");
  const connection = new Connection(rpcUrl, "confirmed");
  const payer = p.publicKey;
  const dest = new PublicKey(depositAddress);
  const tx = new Transaction();
  if (asset === "SOL") {
    tx.add(SystemProgram.transfer({ fromPubkey: payer, toPubkey: dest, lamports: amountBase }));
  } else {
    const src = getAssociatedTokenAddressSync(USDC_MINT, payer);
    const dstAta = getAssociatedTokenAddressSync(USDC_MINT, dest, true);
    // The deposit address is a fresh account, so its USDC token account usually does not exist yet.
    tx.add(createAssociatedTokenAccountIdempotentInstruction(payer, dstAta, dest, USDC_MINT));
    tx.add(createTransferCheckedInstruction(src, USDC_MINT, dstAta, payer, amountBase, 6));
  }
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash("confirmed");
  tx.recentBlockhash = blockhash;
  tx.lastValidBlockHeight = lastValidBlockHeight;
  tx.feePayer = payer;
  const { signature } = await p.signAndSendTransaction(tx);
  return signature;
}
