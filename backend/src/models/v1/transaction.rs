use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Per-user categories file — persisted as user-{uuid}-categories.json
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserCategories {
    pub user_id: Uuid,
    pub expense: Vec<String>,
    pub income: Vec<String>,
}

/// Category type discriminator
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CategoryType {
    Expense,
    Income,
}

// Request DTOs

#[derive(Debug, Clone, Deserialize)]
pub struct PostTransactionRequest {
    pub date: String,
    pub payee: String,
    pub debit_account: String,
    pub credit_account: String,
    pub amount: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTransactionRequest {
    pub date: String,
    pub payee: String,
    pub debit_account: String,
    pub credit_account: String,
    pub amount: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddCategoryRequest {
    pub name: String,
    pub category_type: CategoryType,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteCategoryRequest {
    pub name: String,
    pub category_type: CategoryType,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpeningBalanceRequest {
    pub amount: String,
    pub date: Option<String>,
    pub account_name: Option<String>,
}

// Query parameter DTOs

#[derive(Debug, Clone, Deserialize)]
pub struct ListCategoriesQuery {
    pub r#type: CategoryType,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterQuery {
    pub user: Option<String>,
    pub payee: Option<String>,
    pub begin: Option<String>,
    pub end: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BalanceQuery {
    pub pivot_user: Option<bool>,
    pub user: Option<String>,
}

// Response DTOs

#[derive(Debug, Clone, Serialize)]
pub struct TransactionResponse {
    pub formatted_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransactionPosting {
    pub account: String,
    pub amount: String,
}

/// Structured transaction entry parsed from ledger files. Only transactions
/// that carry an `Id:` tag (posted via this API) are returned — legacy
/// hand-written entries without IDs are skipped so the ID is always stable.
#[derive(Debug, Clone, Serialize)]
pub struct TransactionEntry {
    pub id: Uuid,
    pub date: String,
    pub payee: String,
    pub postings: Vec<TransactionPosting>,
    pub posted_by: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BalanceResponse {
    pub output: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegisterResponse {
    pub output: String,
}


#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Feature: transaction-ledger-api, Property 18: Categories serialization round-trip
    // **Validates: Requirements 9.2**

    /// Strategy for generating valid category name strings.
    /// Categories contain alphanumeric chars, colons, spaces, hyphens, underscores.
    fn category_name_strategy() -> impl Strategy<Value = String> {
        prop::collection::vec(
            prop::char::ranges(vec![
                'a'..='z',
                'A'..='Z',
                '0'..='9',
                ':'..=':',
                ' '..=' ',
                '-'..='-',
                '_'..='_',
            ].into()),
            1..=50,
        )
        .prop_map(|chars| chars.into_iter().collect::<String>())
    }

    fn categories_vec_strategy() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(category_name_strategy(), 0..=10)
    }

    fn uuid_strategy() -> impl Strategy<Value = Uuid> {
        (any::<u128>()).prop_map(|bits| Uuid::from_u128(bits))
    }

    fn user_categories_strategy() -> impl Strategy<Value = UserCategories> {
        (uuid_strategy(), categories_vec_strategy(), categories_vec_strategy()).prop_map(
            |(user_id, expense, income)| UserCategories {
                user_id,
                expense,
                income,
            },
        )
    }

    proptest! {
        #[test]
        fn prop_user_categories_serialization_round_trip(categories in user_categories_strategy()) {
            let json = serde_json::to_string(&categories).expect("serialization should succeed");
            let deserialized: UserCategories =
                serde_json::from_str(&json).expect("deserialization should succeed");
            prop_assert_eq!(categories, deserialized);
        }
    }
}
