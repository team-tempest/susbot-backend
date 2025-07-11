# SusBot: Your Web3 Safety Scanner - Powered by AI

## 1. Project Vision

To provide every Web3 user with a simple, instant, and understandable safety check for any crypto address, token, or smart contract. SusBot acts like an "antivirus for Web3," translating complex on-chain data into a clear "safe" or "risky" verdict, backed by an AI-powered explanation. We protect users from scams, fraudulent tokens, and malicious contracts.

## 2. Use Cases

*   **Primary Use Case:** A user is about to interact with a new token they saw on social media. They are unsure if it's a scam.
    1.  The user visits the SusBot web app.
    2.  They copy the token's contract address and paste it into the input field on SusBot.
    3.  They click the "Scan Now" button.
    4.  Within seconds, SusBot displays a trust score (e.g., 85/100), a clear verdict ("Looks Safe" or "High Risk"), and a list of identified risks in simple language (e.g., "⚠️ The contract owner can mint new tokens," "✅ Contract source code is verified").
    5.  An AI-generated summary explains the technical details in a conversational way, like a friendly expert.

*   **Secondary Use Case:** A user wants to check the reputation of a wallet address they are about to send funds to.
    1.  The user pastes the wallet address into SusBot.
    2.  SusBot checks for known scam associations, transaction history with malicious contracts, and provides a summary of its findings.

## 3. High-Level Architecture

The application is a full-stack dApp running entirely on the Internet Computer. It consists of a React frontend and a Rust backend canister.

*   **Frontend (Asset Canister):** A React application providing the user interface. It is served to the user's browser directly from an ICP asset canister.
*   **Backend (Rust Canister):** The core logic engine written in Rust. This canister exposes public methods that the frontend can call. It is responsible for orchestrating data collection via HTTP outcalls, running security checks, and interfacing with an AI model.
*   **External Services (via HTTP Outcalls):**
    *   **Blockchain Explorer API (e.g., Etherscan):** To fetch on-chain data like contract source code, ABI, transaction history, and token holder information.
    *   **AI Model API (e.g., OpenAI):** To translate technical findings into easy-to-understand explanations for the user.

*(See `ARCHITECTURE.md` for a visual diagram.)*

## 4. Detailed Workflow

1.  **User Interaction:** The user pastes a contract address into the React frontend and clicks "Scan".
2.  **Frontend to Backend Call:** The frontend calls the `analyze_address` method on the backend Rust canister, passing the address as an argument.
3.  **Canister Data Fetching:** The backend canister receives the request and performs an `HTTP outcall` to the Etherscan API to retrieve the smart contract's source code and verification status.
4.  **On-Chain Analysis (in Canister):** The canister's Rust code performs a series of automated checks on the retrieved data:
    *   Is the source code verified?
    *   Does the code contain potentially malicious functions (e.g., a proxy with an unlocked implementation, ability for owner to pause transfers, blacklist users, or mint infinite tokens)?
    *   Is the token distribution heavily concentrated in a few wallets (potential "rug pull" risk)?
    *   Does the address appear on known scam blacklists?
5.  **AI-Powered Explanation:** The canister bundles the findings from its analysis into a structured prompt. It then makes a second `HTTP outcall` to an AI API (e.g., GPT-4). The prompt asks the AI to:
    *   "Explain the following security risks to a non-technical user."
    *   "Provide a final trust score from 0 to 100 based on these factors."
6.  **Receive and Format Response:** The canister receives the structured response from the AI. It combines its own raw findings with the AI's simplified text and score into a final JSON report.
7.  **Return to Frontend:** The backend canister returns the final JSON report to the React frontend.
8.  **Display Results:** The frontend parses the JSON and dynamically renders the results in a user-friendly dashboard, showing the score, risk factors, and the simple AI explanation.

## 5. ICP Features Used

*   **Rust Canister Development Kit (CDK):** For writing the secure, high-performance backend logic.
*   **HTTP Outcalls:** The core feature enabling the canister to interact with off-chain Web2 APIs (Etherscan, AI models) in a trustless way.
*   **Asset Canister:** For hosting and serving the React frontend directly on-chain, creating a fully decentralized application.
*   **Internet Identity:** Can be integrated for future premium features (e.g., saving scan history, setting up alerts).

## 6. Future Plans & Monetization

*   **Freemium Model:** Users get 3-5 free scans per day. A subscription (paid in ICP/ckBTC) unlocks unlimited scans and advanced features.
*   **Advanced Features:**
    *   **Proactive Alerts:** Users can monitor their own wallets and get alerts if they interact with a risky address.
    *   **DApp Browser Extension:** A browser plugin that automatically scans contracts on pages the user visits.
    *   **Multi-Chain Support:** Extend analysis to other blockchains (Solana, BSC, etc.). 