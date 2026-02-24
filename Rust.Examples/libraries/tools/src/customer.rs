#![allow(dead_code)]
use app_dto::customer::{
    credit_score::{CreditScoreOperation, McpCreditScoreToolInput, McpCreditScoreToolOutput},
    customer_information::{
        CustomerInformation, McpCustomerInformationToolInput, McpCustomerInformationToolOutput,
    },
    risk::{McpRiskToolInput, McpRiskToolOutput},
};
use app_error::AppError;
use app_llama_cpp::embedding::embedding;
use app_schema::{customer::Customer, kb::KnowledgeBased};
use app_state::AppState;
use rmcp::{
    ErrorData as McpError,
    handler::server::{
        router::tool::ToolRouter,
        wrapper::{Json, Parameters},
    },
    tool, tool_router,
};
use std::sync::Arc;
#[derive(Clone)]

pub struct CustomerTools {
    state: Arc<AppState>,
    pub tool_router: ToolRouter<Self>,
}

#[tool_router]
impl CustomerTools {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Search list of customers based on name, email, or account number; Providing no filter results list of all customers"
    )]
    async fn get_customer_information(
        &self,
        Parameters(args): Parameters<McpCustomerInformationToolInput>,
    ) -> Result<Json<McpCustomerInformationToolOutput>, McpError> {
        let pg = self.state.pg.clone();
        let mut conditions: Vec<String> = Vec::new();

        if let Some(name) = &args.first_name {
            conditions.push(format!("(first_name ILIKE '%{}%')", &name));
        }
        if let Some(name) = &args.last_name {
            conditions.push(format!("(last_name ILIKE '%{}%')", &name));
        }
        if let Some(account) = &args.account_number {
            conditions.push(format!("(account_number ilike '%{}%')", &account));
        }
        if let Some(email) = &args.email {
            conditions.push(format!("(email ilike '%{}%')", &email));
        }
        let condition_len = conditions.len();
        let query = format!(
            "{} {}",
            &Customer::select_base(),
            match condition_len {
                0 => "".to_owned(),
                _ => format!(
                    " WHERE {}",
                    &conditions
                        .iter()
                        .map(|x| format!("{}", &x))
                        .collect::<Vec<String>>()
                        .join(" AND ")
                ),
            }
        );
        let customers: Vec<Customer> = sqlx::query_as::<_, Customer>(&query)
            .fetch_all(&pg)
            .await
            .map_err(AppError::from)?;

        let mut customer_list: Vec<CustomerInformation> = Vec::new();

        for customer in customers.into_iter() {
            let content_embd = embedding(
                &self.state.config,
                &format!("{} {}", &customer.first_name, &customer.last_name),
            )
            .await
            .map_err(AppError::from)?;

            let public_res = sqlx::query_as::<_, KnowledgeBased>("SELECT id, chunk, created_at FROM rag.knowledge_based WHERE 1 - (embedding <=> $1::vector) >= 0.6 ORDER BY embedding <=> $1::vector LIMIT 5;")
                        .bind(&content_embd)
                        .fetch_all(&pg)
                        .await
                        .map_err(AppError::from)?;
            customer_list.push(CustomerInformation {
                customer_record: customer.clone(),
                public_information: public_res,
            });
        }

        Ok(Json(McpCustomerInformationToolOutput { customer_list }))
    }

    #[tool(
        description = "Search list of customers based on credit score; operation = {LESS, EQUAL, MORE} determines how filter the score"
    )]
    async fn search_customer_by_credit_score(
        &self,
        Parameters(args): Parameters<McpCreditScoreToolInput>,
    ) -> Result<Json<McpCreditScoreToolOutput>, McpError> {
        let pg = self.state.pg.clone();

        let query = format!(
            "{} WHERE credit_score {} {}",
            &Customer::select_base(),
            match &args.operation {
                CreditScoreOperation::LESS => "<",
                CreditScoreOperation::EQUAL => "=",
                CreditScoreOperation::MORE => ">",
            },
            &args.score
        );
        let customer_list: Vec<Customer> = sqlx::query_as::<_, Customer>(&query)
            .fetch_all(&pg)
            .await
            .map_err(AppError::from)?;
        Ok(Json(McpCreditScoreToolOutput { customer_list }))
    }

    #[tool(
        description = "Search list of customers based on risk = {LOW, MEDIUM, HIGH} determines how filter"
    )]
    async fn search_customer_by_risk(
        &self,
        Parameters(args): Parameters<McpRiskToolInput>,
    ) -> Result<Json<McpRiskToolOutput>, McpError> {
        let pg = self.state.pg.clone();

        let query = format!(
            "{} WHERE risk_level ILIKE '{}'",
            &Customer::select_base(),
            &args.risk.to_string(),
        );
        let customer_list: Vec<Customer> = sqlx::query_as::<_, Customer>(&query)
            .fetch_all(&pg)
            .await
            .map_err(AppError::from)?;
        Ok(Json(McpRiskToolOutput { customer_list }))
    }
}
