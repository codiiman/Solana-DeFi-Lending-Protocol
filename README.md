# Solana DeFi Lending Protocol

![Anchor](https://img.shields.io/badge/Anchor-0.30.1-000000?logo=anchor)
![Solana](https://img.shields.io/badge/Solana-1.18-9945FF?logo=solana)
![Rust](https://img.shields.io/badge/Rust-1.70+-000000?logo=rust)

A production-grade Solana DeFi lending protocol clone inspired by [Kamino Finance's K-Lend](https://kamino.finance), built with Anchor 0.30+ and following February 2026 best practices.

## 📋 Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Architecture](#architecture)
- [Interest Rate Model](#interest-rate-model)
- [Liquidation Mechanics](#liquidation-mechanics)
- [Installation](#installation)
- [Building & Testing](#building--testing)
- [Deployment](#deployment)
- [Contact](#contact)

## 🎯 Overview

This project implements a complete DeFi lending protocol for Solana, similar to Kamino Finance's K-Lend. It enables users to:

- **Supply assets** to earn interest via yield-bearing tokens
- **Borrow assets** against collateral with dynamic interest rates
- **Liquidate** unhealthy positions at a discount
- **Create isolated markets** with custom parameters (LTV, liquidation threshold, oracle)
- **Automate yield** through vault strategies

### Key Characteristics

- **Modular Markets**: Each asset has its own isolated lending market with configurable parameters
- **Dynamic Interest Rates**: Utilization-based rates that adjust automatically
- **Yield-Bearing Tokens**: Supply tokens accrue interest automatically
- **Health Factor**: Real-time collateralization ratio monitoring
- **Permissionless Liquidation**: Anyone can liquidate unhealthy positions
- **Vault Strategies**: Automated yield optimization across multiple markets

## ✨ Features

### Core Functionality

1. **Market Creation**
   - Create isolated lending markets for any SPL token
   - Configurable LTV ratios (up to 80%)
   - Custom liquidation thresholds (85-90%)
   - Oracle integration (Pyth or custom)

2. **Supply & Earn**
   - Supply assets to markets
   - Receive yield-bearing tokens (supply tokens)
   - Automatic interest accrual
   - Withdraw anytime (subject to health factor)

3. **Borrow Against Collateral**
   - Borrow up to LTV limit (e.g., 75% of collateral value)
   - Dynamic interest rates based on utilization
   - Health factor monitoring
   - Repay anytime to reduce debt

4. **Liquidation**
   - Permissionless liquidation of unhealthy positions
   - Liquidation bonus (5% discount) for liquidators
   - Seize collateral at favorable rates
   - Protects protocol solvency

5. **Vault Strategies**
   - Conservative: Low risk, stable yields
   - Balanced: Moderate risk/reward
   - Aggressive: Higher risk, higher yields
   - Automatic rebalancing across markets

### Security Features

- ✅ Anchor framework best practices
- ✅ PDA-based account management
- ✅ Health factor checks on all operations
- ✅ Oracle staleness validation
- ✅ Math overflow protection
- ✅ Reentrancy protection via Anchor's account model
- ✅ Custom error types for clear failure cases

## 🏗️ Architecture

### Program Structure

```
solana-defi-lending-protocol/
├── programs/
│   └── solana-defi-lending-protocol/
│       └── src/
│           ├── lib.rs              # Program entry point
│           ├── state.rs            # Account structs
│           ├── errors.rs           # Error definitions
│           ├── constants.rs        # Protocol constants
│           ├── math.rs             # Interest & health calculations
│           └── instructions/
│               ├── mod.rs
│               ├── initialize.rs   # Initialize protocol
│               ├── market.rs        # Create markets
│               ├── supply.rs        # Supply assets
│               ├── borrow.rs        # Borrow assets
│               ├── repay.rs         # Repay debt
│               ├── withdraw.rs      # Withdraw supply
│               ├── liquidate.rs     # Liquidate positions
│               └── vault.rs         # Vault operations
└── tests/
    └── solana-defi-lending-protocol.ts
```

### Account Structure

#### GlobalConfig
- **PDA**: `[b"global_config"]`
- **Fields**:
  - `authority`: Protocol authority
  - `treasury`: Treasury PDA for fees
  - `protocol_fee_bps`: Protocol fee (5% = 500 bps)
  - `market_count`: Total markets created
  - `treasury_bump`: Treasury PDA bump

#### Market
- **PDA**: `[b"market", asset_mint]`
- **Fields**:
  - `market_id`: Unique market identifier
  - `asset_mint`: Underlying asset mint
  - `supply_mint`: Yield-bearing token mint
  - `reserve_vault`: Vault holding supplied assets
  - `oracle`: Price oracle account
  - `ltv_bps`: Loan-to-value ratio (e.g., 7500 = 75%)
  - `liquidation_threshold_bps`: Liquidation threshold (e.g., 8500 = 85%)
  - `total_supplied`: Total assets supplied (with interest)
  - `total_borrowed`: Total assets borrowed (with interest)
  - `total_supply_tokens`: Total supply tokens minted
  - `cumulative_borrow_rate`: For interest accrual
  - `cumulative_supply_rate`: For interest accrual
  - `last_accrual_timestamp`: Last interest accrual time

#### BorrowPosition
- **PDA**: `[b"borrow_position", user, market]`
- **Fields**:
  - `user`: Borrower address
  - `market`: Market address
  - `borrowed_amount`: Principal borrowed
  - `cumulative_borrow_rate_snapshot`: Rate when borrowed
  - `created_at`: Position creation time
  - `last_updated`: Last update time

#### Vault
- **PDA**: `[b"vault", owner]`
- **Fields**:
  - `owner`: Vault owner
  - `strategy`: Strategy type (0=Conservative, 1=Balanced, 2=Aggressive)
  - `total_assets`: Assets under management
  - `allocations`: Market allocation percentages
  - `last_rebalance`: Last rebalance time
  - `rebalance_threshold_bps`: Rebalance trigger threshold

### Instruction Flow

```
1. Initialize Protocol
   └─> Creates GlobalConfig and Treasury PDAs

2. Create Market
   └─> Creates Market account with config
   └─> Sets up reserve vault and supply mint

3. Supply Assets
   └─> Transfer assets to reserve vault
   └─> Mint supply tokens to user
   └─> Update total_supplied

4. Borrow Assets
   └─> Check health factor
   └─> Transfer assets from reserve
   └─> Create/update borrow position
   └─> Update total_borrowed

5. Repay Debt
   └─> Transfer assets to reserve
   └─> Update borrow position
   └─> Update total_borrowed

6. Withdraw Supply
   └─> Check health factor
   └─> Burn supply tokens
   └─> Transfer assets from reserve
   └─> Update total_supplied

7. Liquidate
   └─> Verify health factor < threshold
   └─> Repay debt at discount
   └─> Seize collateral with bonus
   └─> Update both markets
```

## 📊 Interest Rate Model

### Utilization-Based Rates

The protocol uses a **piecewise linear interest rate model** based on utilization:

```
Utilization = Total Borrowed / Total Supplied
```

### Borrow Rate Calculation

**Below Optimal Utilization (≤80%):**
```
borrow_rate = base_rate + slope1 * (utilization / optimal_utilization)
```

**Above Optimal Utilization (>80%):**
```
borrow_rate = base_rate + slope1 + slope2 * ((utilization - optimal) / (1 - optimal))
```

### Supply Rate Calculation

```
supply_rate = borrow_rate * utilization * (1 - protocol_fee)
```

### Rate Parameters

- **Base Rate**: ~2% APY (634,195,839 per second)
- **Slope 1**: ~10% APY per 10% utilization (below optimal)
- **Slope 2**: ~100% APY per 10% utilization (above optimal)
- **Optimal Utilization**: 80%
- **Protocol Fee**: 5% of interest

### Interest Accrual

Interest accrues continuously using compound interest:

```
new_amount = old_amount * (1 + rate_per_second) ^ seconds_elapsed
```

For on-chain efficiency, simplified to:

```
new_amount = old_amount * (1 + rate * seconds / scale)
```

## 💰 Liquidation Mechanics

### Health Factor

Health factor determines if a position can be liquidated:

```
Health Factor = (Collateral Value * Liquidation Threshold) / Total Borrowed Value
```

- **Health Factor > 1.0**: Position is safe
- **Health Factor < 1.0**: Position can be liquidated

### Liquidation Process

1. **Detection**: Health factor drops below threshold (1.0)
2. **Liquidation**: Anyone can repay debt and seize collateral
3. **Bonus**: Liquidator receives 5% discount on seized collateral
4. **Protection**: Protocol remains solvent

### Liquidation Formula

```
collateral_seized = debt_repaid * (1 + liquidation_bonus) * (collateral_price / borrow_price)
```

Where:
- `liquidation_bonus` = 5% (500 basis points)
- Prices from oracles

### Example

- User borrows 100 USDC against 150 SOL collateral
- SOL price drops → Health factor < 1.0
- Liquidator repays 100 USDC
- Receives ~105 USDC worth of SOL (5% bonus)

## 🚀 Installation

### Prerequisites

- Rust 1.70+
- Solana CLI 1.18+
- Anchor CLI 0.30+
- Node.js 18+ and Yarn/npm

### Setup

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd Solana-DeFi-Lending-Protocol
   ```

2. **Install Anchor**
   ```bash
   cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
   avm install latest
   avm use latest
   ```

3. **Install dependencies**
   ```bash
   anchor build
   cd tests && yarn install
   ```

## 🔨 Building & Testing

### Build the Program

```bash
anchor build
```

This will:
- Compile the Rust program
- Generate the IDL
- Create the program binary

### Run Tests

```bash
anchor test
```

Or run tests with verbose output:

```bash
anchor test --skip-local-validator
```

### Test on Localnet

1. **Start local validator**
   ```bash
   solana-test-validator
   ```

2. **Deploy program**
   ```bash
   anchor deploy
   ```

3. **Run tests**
   ```bash
   anchor test --skip-local-validator
   ```

### Test Coverage

The test suite includes:
- ✅ Protocol initialization
- ✅ Market creation
- ✅ Supply operations
- ✅ Borrow operations
- ✅ Repay operations
- ✅ Withdraw operations
- ✅ Liquidation scenarios
- ✅ Interest accrual over time
- ✅ Health factor calculations

## 🌐 Deployment

### Devnet Deployment

1. **Set Solana CLI to devnet**
   ```bash
   solana config set --url devnet
   ```

2. **Airdrop SOL (if needed)**
   ```bash
   solana airdrop 2 <your-wallet-address>
   ```

3. **Update program ID**
   - Generate new keypair: `solana-keygen new -o target/deploy/solana_defi_lending_protocol-keypair.json`
   - Update `declare_id!` in `lib.rs`
   - Update `Anchor.toml` with new program ID

4. **Build and deploy**
   ```bash
   anchor build
   anchor deploy
   ```

### Mainnet Deployment

⚠️ **WARNING**: Only deploy to mainnet after thorough auditing and testing.

1. **Set Solana CLI to mainnet**
   ```bash
   solana config set --url mainnet-beta
   ```

2. **Build for mainnet**
   ```bash
   anchor build
   ```

3. **Deploy (requires sufficient SOL)**
   ```bash
   anchor deploy
   ```

4. **Verify deployment**
   ```bash
   solana program show <program-id>
   ```

### Post-Deployment

1. **Initialize protocol**
   - Call `initialize` instruction with protocol authority
   - Sets up GlobalConfig and Treasury PDAs

2. **Create markets**
   - Call `create_market` for each asset
   - Configure LTV, liquidation threshold, oracle

3. **Verify setup**
   - Check GlobalConfig account
   - Verify markets are created correctly
   - Test supply/borrow operations

## 📧 Contact

- Telegram: https://t.me/codiiman
- Twitter: https://x.com/codiiman_

