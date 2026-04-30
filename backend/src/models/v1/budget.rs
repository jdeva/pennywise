use serde::{Deserialize, Serialize};

// Domain model — represents a single budget definition (periodic transaction)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetDefinition {
    pub period: String,
    pub account: String,
    pub amount: f64,
    pub currency: String,
}

// Request DTOs
#[derive(Debug, Clone, Deserialize)]
pub struct CreateBudgetRequest {
    pub period: String,
    pub account: String,
    pub amount: String,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateBudgetRequest {
    pub period: String,
    pub account: String,
    pub amount: String,
    pub currency: Option<String>,
}

// Query DTOs
#[derive(Debug, Clone, Deserialize)]
pub struct BudgetReportQuery {
    pub begin: Option<String>,
    pub end: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ForecastQuery {
    pub end_date: Option<String>,
}

// Response DTOs
#[derive(Debug, Clone, Serialize)]
pub struct BudgetDefinitionResponse {
    pub formatted_text: String,
    pub definition: BudgetDefinition,
}

#[derive(Debug, Clone, Serialize)]
pub struct BudgetReportResponse {
    pub output: String,
}
