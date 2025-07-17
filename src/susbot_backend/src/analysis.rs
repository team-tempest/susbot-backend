use regex::Regex;
use serde_derive::Serialize;

#[derive(Clone, Debug, Serialize)]
pub enum RiskLevel {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl RiskLevel {
    fn to_string(&self) -> &'static str {
        match self {
            RiskLevel::Critical => "Critical",
            RiskLevel::High => "High",
            RiskLevel::Medium => "Medium",
            RiskLevel::Low => "Low",
            RiskLevel::Info => "Info",
        }
    }
}

pub struct AnalysisCheck {
    pub name: &'static str,
    pub description: &'static str,
    pub pattern: &'static str,
    pub risk_level: RiskLevel,
    pub score_impact: i32,
}

#[derive(Debug, Serialize)]
pub struct FoundRisk {
    pub check_name: &'static str,
    pub description: &'static str,
    pub risk_level: RiskLevel,
}

impl FoundRisk {
    pub fn to_string(&self) -> String {
        format!(
            "[{}] {}: {}",
            self.risk_level.to_string(),
            self.check_name,
            self.description
        )
    }
}

#[derive(Debug, Serialize)]
pub struct AnalysisResult {
    pub score: u8,
    pub risks: Vec<FoundRisk>,
    pub contract_traits: ContractTraits,
}

#[derive(Debug, Serialize)]
pub struct ContractTraits {
    pub verified: bool,
    pub good_distribution: bool,
    pub contract_type: String,
}

impl ContractTraits {
    pub fn new(verified: bool) -> Self {
        Self {
            verified,
            good_distribution: true, // Default to true, this would need token analysis in a real implementation
            contract_type: "Unknown".to_string(), // Default type
        }
    }
}

pub fn analyze_source_code(source_code: &str) -> AnalysisResult {
    let mut score: i32 = 100;
    let mut risks = Vec::new();

    examine_checks(source_code, &mut score, &mut risks);

    prevent_negative_scores(&mut score);

    // Determine contract type from source code
    let contract_type = determine_contract_type(source_code);
    
    let contract_traits = ContractTraits {
        verified: true, // If we have source code, it's verified
        good_distribution: true, // This would need blockchain analysis in a real implementation
        contract_type,
    };

    AnalysisResult {
        score: score as u8,
        risks,
        contract_traits,
    }
}

pub fn analyze_source_code_with_verification(source_code: &str, verified: bool) -> AnalysisResult {
    let mut score: i32 = 100;
    let mut risks = Vec::new();

    examine_checks(source_code, &mut score, &mut risks);

    prevent_negative_scores(&mut score);

    // Determine contract type from source code
    let contract_type = determine_contract_type(source_code);
    
    let contract_traits = ContractTraits {
        verified,
        good_distribution: true, // This would need blockchain analysis in a real implementation
        contract_type,
    };

    AnalysisResult {
        score: score as u8,
        risks,
        contract_traits,
    }
}

fn determine_contract_type(source_code: &str) -> String {
    if source_code.contains("ERC20") || source_code.contains("IERC20") {
        "ERC20 Token".to_string()
    } else if source_code.contains("ERC721") || source_code.contains("IERC721") {
        "NFT (ERC721)".to_string()
    } else if source_code.contains("ERC1155") || source_code.contains("IERC1155") {
        "Multi-Token (ERC1155)".to_string()
    } else if source_code.contains("Ownable") {
        "Ownable Contract".to_string()
    } else if source_code.contains("contract") {
        "Smart Contract".to_string()
    } else {
        "Unknown".to_string()
    }
}

fn examine_checks(source_code: &str, score: &mut i32, risks: &mut Vec<FoundRisk>) {
    for check in CHECKS.iter() {
        let re = Regex::new(check.pattern).unwrap_or_else(|_| panic!("Invalid regex pattern: {}", check.pattern));
        if re.is_match(source_code) {
            *score -= check.score_impact;
            risks.push(FoundRisk {
                check_name: check.name,
                description: check.description,
                risk_level: check.risk_level.clone(),
            });
        }
    }
}

fn prevent_negative_scores(score: &mut i32) {
    if *score < 0 {
        *score = 0;
    }
}

pub const CHECKS: [AnalysisCheck; 16] = [
    AnalysisCheck {
        name: "Self-Destruct",
        description: "The contract can be destroyed by its owner, removing it from the blockchain and sending all its funds to a designated address.",
        pattern: r"selfdestruct\s*\(|suicide\s*\(",
        risk_level: RiskLevel::Critical,
        score_impact: 25,
    },
    AnalysisCheck {
        name: "Delegate Call",
        description: "Unsafe use of 'delegatecall' can lead to unexpected code execution and security vulnerabilities.",
        pattern: r"\.delegatecall\b",
        risk_level: RiskLevel::Critical,
        score_impact: 40,
    },
    AnalysisCheck {
        name: "tx.origin Authentication",
        description: "Using 'tx.origin' for authentication is unsafe and can make the contract vulnerable to phishing attacks.",
        pattern: r"\btx\.origin\b",
        risk_level: RiskLevel::High,
        score_impact: 30,
    },
    AnalysisCheck {
        name: "Block Timestamp Dependency",
        description: "The contract's logic depends on 'block.timestamp', which can be manipulated by miners.",
        pattern: r"\bblock\.timestamp\b",
        risk_level: RiskLevel::Medium,
        score_impact: 15,
    },
    AnalysisCheck {
        name: "Inline Assembly",
        description: "Use of inline assembly ('assembly') bypasses compiler safety checks and requires careful review.",
        pattern: r"\bassembly\b",
        risk_level: RiskLevel::Medium,
        score_impact: 5,
    },
    AnalysisCheck {
        name: "Low-level Call",
        description: "Low-level '.call' is used. If the return value is not checked, it can lead to failed external calls being missed.",
        pattern: r"\.call\b",
        risk_level: RiskLevel::Low,
        score_impact: 5,
    },
    AnalysisCheck {
        name: "Outdated Compiler Version",
        description: "An old Solidity version is used (pragma < 0.8.0), which lacks built-in overflow/underflow checks.",
        pattern: r"pragma\s+solidity\s*[\^<>=]*\s*0\.[0-7]\.",
        risk_level: RiskLevel::Low,
        score_impact: 5,
    },
    AnalysisCheck {
        name: "Unprotected Ether Withdrawal",
        description: "A 'transfer' function is used. If protections like 'require(msg.sender == owner)' are missing, it could be a vulnerability.",
        pattern: r"\.transfer\(",
        risk_level: RiskLevel::High,
        score_impact: 25,
    },
    AnalysisCheck {
        name: "Mint Function",
        description: "The contract contains a 'mint' function, indicating it can create new tokens.",
        pattern: r"\bmint\b",
        risk_level: RiskLevel::Info,
        score_impact: 0,
    },
    AnalysisCheck {
        name: "Infinite Minting",
        description: "The contract allows unlimited token minting without proper restrictions, potentially leading to hyperinflation.",
        pattern: r"function\s+mint\s*\([^)]*\)",
        risk_level: RiskLevel::Critical,
        score_impact: 25,
    },
    AnalysisCheck {
        name: "Owner Can Blacklist Users",
        description: "The contract owner can blacklist or freeze user addresses, giving centralized control over user funds.",
        pattern: r"blacklist|blacklisted|frozen|freeze",
        risk_level: RiskLevel::High,
        score_impact: 15,
    },
    AnalysisCheck {
        name: "Pausable Contract",
        description: "The contract can be paused by the owner, potentially stopping all transfers and functionality.",
        pattern: r"Pausable|_pause\s*\(|_unpause\s*\(|whenNotPaused|whenPaused",
        risk_level: RiskLevel::Medium,
        score_impact: 10,
    },
    AnalysisCheck {
        name: "Missing Event Emits",
        description: "Critical functions lack event emissions, reducing transparency and making it harder to track important actions.",
        pattern: r"function\s+transfer\s*\([^)]*\)",
        risk_level: RiskLevel::Low,
        score_impact: 3,
    },
    AnalysisCheck {
        name: "Hardcoded Gas Limits",
        description: "The contract uses hardcoded gas values which may cause issues if network conditions change.",
        pattern: r"\.gas\s*\(\s*\d+\s*\)",
        risk_level: RiskLevel::Low,
        score_impact: 2,
    },
    AnalysisCheck {
        name: "Unchecked Call Return",
        description: "Low-level calls are made without checking their return values, potentially missing failed calls.",
        pattern: r"\.call\s*\([^)]*\)\s*;",
        risk_level: RiskLevel::Medium,
        score_impact: 10,
    },
    AnalysisCheck {
        name: "Reentrancy Vulnerability",
        description: "The contract may be vulnerable to reentrancy attacks due to external calls before state changes.",
        pattern: r"\.call\s*\([^)]*\)",
        risk_level: RiskLevel::Critical,
        score_impact: 30,
    },
]; 