import { encodeFunctionData, parseAbi, stringToHex, type Hex } from "viem";

interface Eip1193 {
  request(args: { method: string; params?: unknown[] }): Promise<unknown>;
}

export function getEthereum(): Eip1193 | null {
  return (window as unknown as { ethereum?: Eip1193 }).ethereum ?? null;
}

export async function connectEthereum(): Promise<string> {
  const eth = getEthereum();
  if (!eth) throw new Error("MetaMask not found");
  const accounts = (await eth.request({ method: "eth_requestAccounts" })) as string[];
  if (!accounts[0]) throw new Error("no account");
  // Maya ETH vaults live on Ethereum mainnet.
  await eth.request({ method: "wallet_switchEthereumChain", params: [{ chainId: "0x1" }] }).catch(() => undefined);
  return accounts[0];
}

const ROUTER_ABI = parseAbi([
  "function depositWithExpiry(address vault, address asset, uint256 amount, string memo, uint256 expiration) payable",
]);
const ERC20_ABI = parseAbi(["function approve(address spender, uint256 amount) returns (bool)"]);

function hex(n: bigint): Hex {
  return `0x${n.toString(16)}` as Hex;
}

/** Native ETH to the Maya inbound address with the memo as calldata. Returns the tx hash. */
export async function sendEthNative(from: string, inboundAddress: string, amountWei: bigint, memo: string): Promise<string> {
  const eth = getEthereum();
  if (!eth) throw new Error("MetaMask not found");
  return (await eth.request({
    method: "eth_sendTransaction",
    params: [{ from, to: inboundAddress, value: hex(amountWei), data: stringToHex(memo) }],
  })) as string;
}

/** ERC-20 (USDC) via the Maya router: approve, then depositWithExpiry. Returns the deposit tx hash. */
export async function sendErc20ViaRouter(
  from: string,
  router: string,
  vault: string,
  token: string,
  amount: bigint,
  memo: string,
  expiry: number,
): Promise<string> {
  const eth = getEthereum();
  if (!eth) throw new Error("MetaMask not found");
  await eth.request({
    method: "eth_sendTransaction",
    params: [{ from, to: token, data: encodeFunctionData({ abi: ERC20_ABI, functionName: "approve", args: [router as Hex, amount] }) }],
  });
  return (await eth.request({
    method: "eth_sendTransaction",
    params: [
      {
        from,
        to: router,
        data: encodeFunctionData({
          abi: ROUTER_ABI,
          functionName: "depositWithExpiry",
          args: [vault as Hex, token as Hex, amount, memo, BigInt(expiry)],
        }),
      },
    ],
  })) as string;
}
