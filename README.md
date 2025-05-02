"""# ✨ Belactokenlauncher ✨

A Solana smart contract for launching SPL tokens via a bonding curve mechanism, built with native Rust and SPL.

---

## Overview

**Belactokenlauncher** provides the on-chain foundation for creating and trading new SPL tokens using a constant-product bonding curve. Users can deploy this contract to enable:

*   **Fair Launch:** Tokens are minted and made available for purchase directly through the curve.
*   **Automated Liquidity:** The curve itself acts as the initial market maker.
*   **Configurable Trading:** Buy/sell actions are mediated by the contract, with an optional, updateable trade fee.
*   **Path to DEX:** Includes functionality designed to facilitate migrating liquidity to a standard Serum/OpenBook market and Raydium AMM pool.

This contract is developed using native Rust and the Solana Program Library (SPL), offering granular control but requiring careful implementation and client-side handling.

## Features

*   🚀 **SPL Token Creation:** Generates a new Token Mint and associated Metaplex Metadata.
*   📈 **Constant-Product Bonding Curve:** Manages SOL deposits and token distribution based on virtual reserves.
*   💸 **Dynamic Trade Fees:** A designated authority can update the `trade_fee_basis_points` after deployment.
*   🔒 **Migration Mechanism:** Supports locking the curve and transferring locked SOL/Token liquidity (intended for backend orchestration).
*   🦀 **Native Rust & SPL:** No Anchor framework dependency.

## Getting Started

### Prerequisites

*   **Rust:** Latest stable version recommended. Install via [rustup.rs](https://rustup.rs/).
*   **Solana Tool Suite:** Latest version recommended. [Installation Guide](https://docs.solana.com/cli/install).
*   **(Windows)** **WSL:** Strongly recommended for building Solana programs on Windows.

### Building the Contract

1.  **Clone the repository** (if you haven't already).
2.  **Navigate to the contract directory:**
    ```bash
    cd tokenlaunchcontract
    ```
3.  **Build the BPF bytecode:**
    ```bash
    cargo build-sbf
    ```
    > **Note:** If on Windows, run the build command within your WSL environment.

    The compiled program artifact will be located at `target/deploy/tokenlaunchcontract.so`.

### Deployment

1.  **Prepare Deployer Wallet:** Ensure you have a Solana keypair funded with SOL on your target network (Devnet, Testnet, or Mainnet).
    *   Generate: `solana-keygen new`
    *   Airdrop (Devnet/Testnet): `solana airdrop 2 <YOUR_WALLET_ADDRESS> --url <NETWORK_URL>`
2.  **Deploy:**
    ```bash
    solana program deploy target/deploy/tokenlaunchcontract.so \
      --keypair <PATH_TO_YOUR_KEYPAIR.json> \
      --url <NETWORK_URL>
    ```
    *   Replace `<PATH_TO_YOUR_KEYPAIR.json>` with the path to your funded deployer wallet.
    *   Set `<NETWORK_URL>`:
        *   Devnet: `d` or `https://api.devnet.solana.com`
        *   Testnet: `t` or `https://api.testnet.solana.com`
        *   Mainnet: `m` or `https://api.mainnet-beta.solana.com`
3.  **Record Program ID:** The CLI will output the deployed **Program ID**. Save this ID – it's essential for interacting with your contract.

## ⚠️ Disclaimer

This smart contract is provided "as is" without warranty of any kind. It has **not undergone a formal security audit**. Deploying and interacting with this contract involves significant risk, especially on Mainnet with real assets.

**Always conduct thorough testing and consider a professional security audit before production use.**

## 📜 License

This project is licensed under the [**INSERT LICENSE HERE**] License. (E.g., MIT, Apache 2.0). Please add a `LICENSE` file to the repository root. 