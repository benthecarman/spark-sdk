//! Instant static deposits. Authentication uses the normal SSP session client.
use super::{CurrencyAmount, ServiceProvider, ServiceProviderError};
use graphql_client::{GraphQLQuery, QueryBody};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstantDepositQuote {
    pub id: String,
    pub network: String,
    pub transaction_id: String,
    pub output_index: u32,
    pub deposit_amount: CurrencyAmount,
    pub credit_amount: CurrencyAmount,
    pub quote_signature: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstantDepositPlan {
    pub id: String,
    pub amount: CurrencyAmount,
    pub confirmations: u32,
    pub status: String,
    pub transfer_spark_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstantDepositQuoteResponse {
    pub quote: InstantDepositQuote,
    pub fulfillment_plans: Vec<InstantDepositPlan>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstantDepositClaimResponse {
    pub claim_id: String,
}

struct QuoteQuery;
#[derive(Deserialize)]
struct QuoteData {
    create_instant_static_deposit_quote: InstantDepositQuoteResponse,
}
impl GraphQLQuery for QuoteQuery {
    type Variables = Value;
    type ResponseData = QuoteData;
    fn build_query(variables: Value) -> QueryBody<Value> {
        QueryBody {
            variables,
            operation_name: "CreateInstantStaticDepositQuote",
            query: r#"
mutation CreateInstantStaticDepositQuote($transaction_id:String!,$output_index:Int!,$network:BitcoinNetwork!) {
 create_instant_static_deposit_quote(input:{transaction_id:$transaction_id,output_index:$output_index,network:$network}) {
  quote { id network transaction_id output_index quote_signature deposit_amount { ...Amount } credit_amount { ...Amount } }
  fulfillment_plans { id amount { ...Amount } confirmations status transfer_spark_id }
 }
}
fragment Amount on CurrencyAmount { original_value original_unit preferred_currency_unit preferred_currency_value_rounded }
"#,
        }
    }
}
struct ClaimQuery;
#[derive(Deserialize)]
struct ClaimData {
    create_claim_instant_static_deposit: InstantDepositClaimResponse,
}
impl GraphQLQuery for ClaimQuery {
    type Variables = Value;
    type ResponseData = ClaimData;
    fn build_query(variables: Value) -> QueryBody<Value> {
        QueryBody {
            variables,
            operation_name: "ClaimInstantStaticDeposit",
            query: r#"
mutation ClaimInstantStaticDeposit($static_deposit_quote_id:ID!,$static_deposit_address_private_key_share:String!,$signature:String!) {
 create_claim_instant_static_deposit(input:{static_deposit_quote_id:$static_deposit_quote_id,static_deposit_address_private_key_share:$static_deposit_address_private_key_share,signature:$signature}) { claim_id }
}"#,
        }
    }
}
impl ServiceProvider {
    pub async fn get_instant_deposit_quote(
        &self,
        transaction_id: &str,
        output_index: u32,
        network: &str,
    ) -> Result<InstantDepositQuoteResponse, ServiceProviderError> {
        Ok(self.gql_client.post_query::<QuoteQuery,_>(json!({"transaction_id":transaction_id,"output_index":output_index,"network":network})).await?.create_instant_static_deposit_quote)
    }
    pub async fn claim_instant_deposit(
        &self,
        quote_id: &str,
        key: &[u8],
        signature: &[u8],
    ) -> Result<InstantDepositClaimResponse, ServiceProviderError> {
        Ok(self.gql_client.post_query::<ClaimQuery,_>(json!({"static_deposit_quote_id":quote_id,"static_deposit_address_private_key_share":hex::encode(key),"signature":hex::encode(signature)})).await?.create_claim_instant_static_deposit)
    }
}

impl InstantDepositQuote {
    pub fn user_statement(&self, address: &str) -> Result<Vec<u8>, ServiceProviderError> {
        let signature = hex::decode(&self.quote_signature)
            .map_err(|e| ServiceProviderError::ParseError(e.to_string()))?;
        Ok(crate::utils::tagged_hasher::TaggedHasher::new(&[
            "spark",
            "claim_instant_static_deposit",
        ])
        .add_string(&self.network.to_lowercase())
        .add_u64(3)
        .add_u64(self.credit_amount.as_sats()?)
        .add_u64(0)
        .add_string(address)
        .add_u64(self.deposit_amount.as_sats()?)
        .add_bytes(&signature)
        .signable_message())
    }
}
