use regex::Regex;

#[derive(Clone, Debug)]
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

#[derive(Debug)]
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

pub struct AnalysisResult {
    pub score: u8,
    pub risks: Vec<FoundRisk>,
}

pub fn analyze_source_code(source_code: &str) -> AnalysisResult {
    let mut score: i32 = 100;
    let mut risks = Vec::new();

    examine_checks(source_code, &mut score, &mut risks);

    prevent_negative_scores(&mut score);

    AnalysisResult {
        score: score as u8,
        risks,
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

pub const CHECKS: [AnalysisCheck; 9] = [
    AnalysisCheck {
        name: "Self-Destruct",
        description: "The contract can be destroyed by its owner, removing it from the blockchain and sending all its funds to a designated address.",
        pattern: r"selfdestruct\s*\(|suicide\s*\(",
        risk_level: RiskLevel::Critical,
        score_impact: 70,
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
        pattern: r"pragma\s+solidity\s*[\^<>=]*\s*0\.[4-7]\.",
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
]; 