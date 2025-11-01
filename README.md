# NOVA Insurance Protocol

**Decentralized Micro Insurance on Solana**

NOVA is a decentralized micro-insurance protocol making disaster protection accessible to everyone. Built on Solana for speed and low fees, and powered by Switchboard VRF for provable fairness, NOVA lets anyone create or join affordable insurance pools with transparent, community-verified claims and instant USDC payouts — no volatility, no middlemen.

## The Problem

Traditional insurance excludes billions of people due to high premiums, complex paperwork, and centralized control. In developing markets, over 90% of the population lacks access to catastrophic coverage for health emergencies, natural disasters, or crop failures.

## Our Solution

NOVA enables anyone to create or join micro-insurance pools with monthly premiums as low as $1-5. When disaster strikes, community validators—randomly selected via VRF—verify claims without bias. If multiple valid claims exceed available funds, VRF ensures fair distribution. No middlemen, no denial letters, no waiting months for payouts. Using USDC eliminates volatility risk, making coverage predictable and globally accessible while enabling seamless cross-border payouts without intermediaries.

### Protocol Flow

![NOVA Insurance Protocol Flow](https://github.com/dhiraj0911/nova-insurance/blob/main/app/public/flow.png?raw=true)

The diagram above illustrates the complete architecture of NOVA Insurance, organized into six interconnected layers:

- **Layer 1: Pool Management** - Create and join insurance pools with USDC-based premiums
- **Layer 2: Validator Management** - Stake-based validator system with reputation tracking
- **Layer 3: Claims Management** - Submit claims with evidence and status tracking
- **Layer 4: VRF Randomness** - Switchboard VRF for fair validator selection and claim distribution
- **Layer 5: Distribution & Payout** - Queue management and USDC payouts to claimants
- **Layer 6: Yield Optimization** - Kamino Finance integration for idle fund yield generation

---

## Core Features

### 1. **Flexible Insurance Pools**
Create pools for specific needs: medical emergencies, weather events, crop insurance, or general coverage. Pool creators set premium amounts, coverage limits, and claim validation requirements.

### 2. **Community Validation**
Validators stake SOL to participate in claim verification. VRF randomly assigns validators to each claim, preventing collusion. Correct validations earn reputation and fees; dishonest votes result in stake slashing.

### 3. **Fair Distribution**
When claims exceed pool funds (oversubscription), VRF randomly selects which claims receive payouts. Future versions will prioritize by medical urgency, payment history, and time in queue.

### 4. **Yield Generation**
Idle pool funds are deposited into Kamino vaults to earn yield, increasing pool sustainability without raising premiums.

### 5. **On-Chain Transparency**
Every premium, claim, validation, and payout is recorded on Solana. Users can verify pool health, validator reputation, and distribution fairness at any time.

---

## Technical Architecture

### Smart Contract Instructions

#### Pool Management
- `initialize_pool` - Create new insurance pool with USDC vault
- `join_pool` - Users join and pay first premium
- `pay_premium` - Monthly premium payments to maintain coverage

#### Claims Processing
- `submit_claim` - File claim with incident details and evidence
- `validate_claim` - Validators vote to approve/reject claims

#### Validator System
- `stake_as_validator` - Stake 0.1+ SOL to become validator
- `initialize_validator_registry` - Setup validator tracking for pool

#### VRF Integration (Switchboard)
- `initialize_vrf_state` - Setup VRF for pool
- `request_validator_selection` - Trigger VRF for random validator assignment
- `fulfill_validator_selection` - VRF callback assigns validators to claims

#### Distribution & Payouts
- `initialize_distribution_queue` - Setup payout queue
- `add_to_distribution_queue` - Queue approved claims
- `distribute_claims` - Select claims for payout (VRF if oversubscribed)
- `payout_claim` - Execute USDC transfer to claimant

#### Yield Generation (Kamino)
- `deposit_to_yield` - Move idle funds to Kamino vault
- `withdraw_from_yield` - Retrieve funds for claim payouts

### Account Structure

**InsurancePool** - Pool configuration, statistics, vault address  
**UserCoverage** - Individual user's coverage status and payment history  
**ClaimRequest** - Claim details, validation votes, status tracking  
**ValidatorStake** - Validator reputation, stake amount, validation history  
**ValidatorRegistry** - Pool's active validator list (max 100)  
**VrfState** - VRF request tracking for validator selection  
**DistributionQueue** - Approved claims awaiting payout  

### Key Mechanisms

**Fraud Prevention**: Claims must be filed within the pool's claim period and cannot predate the user's join date.

**Reputation System**: Validators start at 5000/10000 reputation. Voting with the majority adds +100; voting against majority subtracts -200 and slashes stake by (min_validators × 2%).

**Claim Status Flow**: `Pending` → `UnderValidation` → `Approved` → `Queued` → `Distributed` (or `Rejected`)

**VRF Randomness**: Used twice—once to select which validators review a claim, and again (if needed) to fairly distribute payouts when claims exceed pool funds.

---

## Getting Started

### Prerequisites
- Rust 1.70+
- Solana CLI 1.18+
- Anchor 0.28.0
- Node.js 16+

### Installation

```bash
# Clone repository
git clone https://github.com/yourusername/nova-insurance
cd nova-insurance

# Install dependencies
yarn install

# Build program
anchor build

# Run tests
anchor test --skip-local-validator
```

### Deployment

```bash
# Deploy to devnet
anchor deploy --provider.cluster devnet

# Update program ID in lib.rs and Anchor.toml
anchor keys list
```

### Configuration

Update `Anchor.toml` with your:
- Solana RPC endpoint
- Wallet path
- Program ID

---

## Usage Example

```typescript
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { NovaInsurance } from "../target/types/nova_insurance";

// Initialize pool for medical emergencies
await program.methods
  .initializePool(
    { medical: {} },  // pool_type
    5_000_000,        // 5 USDC premium
    50_000_000,       // 50 USDC max coverage
    3,                // minimum 3 validators
    2_592_000         // 30-day claim window
  )
  .accounts({ ... })
  .rpc();

// User joins pool
await program.methods
  .joinPool(50_000_000) // 50 USDC coverage
  .accounts({ ... })
  .rpc();

// Submit claim
await program.methods
  .submitClaim(
    { medicalEmergency: {} },
    25_000_000,  // 25 USDC requested
    incidentTimestamp,
    "ipfs://QmHash..." // evidence hash
  )
  .accounts({ ... })
  .rpc();
```

---

## Technology Stack

- **Blockchain**: Solana (Devnet currently)
- **Framework**: Anchor 0.28.0
- **Language**: Rust (on-chain) + TypeScript (tests)
- **Token Standard**: SPL Token (USDC- Stablecoin)
- **Randomness**: Switchboard VRF
- **Yield**: Kamino Finance integration (in progress)

---

## Contributing

This is a hackathon project currently under active development. Contributions, suggestions, and feedback are welcome! Please open an issue or submit a pull request.

---

## License

ISC License - See LICENSE file for details

---

## Contact & Links

**Program ID (Devnet)**: `4iAKZaYASzqvW17iaZLZCxDxNTYCEJn4STL9RVdqC9V8`

Built with ❤️ for the Solana ecosystem
