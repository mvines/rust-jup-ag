use {
    base64::prelude::{Engine as _, BASE64_STANDARD},
    itertools::Itertools,
    serde::{Deserialize, Serialize},
    solana_sdk::transaction::VersionedTransaction,
    solana_sdk::{
        instruction::Instruction,
        pubkey::{ParsePubkeyError, Pubkey},
    },
    std::{collections::HashMap, env, fmt, str::FromStr},
};

mod field_as_string;
mod field_instruction;
mod field_prioritization_fee;
mod field_pubkey;

/// A `Result` alias where the `Err` case is `jup_ag::Error`.
pub type Result<T> = std::result::Result<T, Error>;

// Reference: https://quote-api.jup.ag/v4/docs/static/index.html
fn quote_api_url() -> String {
    env::var("QUOTE_API_URL").unwrap_or_else(|_| "https://quote-api.jup.ag/v6".to_string())
}

// Reference: https://quote-api.jup.ag/docs/static/index.html
fn price_api_url() -> String {
    env::var("PRICE_API_URL").unwrap_or_else(|_| "https://api.jup.ag/price/v2".to_string())
}

/// The Errors that may occur while using this crate
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("reqwest: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("invalid pubkey in response data: {0}")]
    ParsePubkey(#[from] ParsePubkeyError),

    #[error("base64: {0}")]
    Base64Decode(#[from] base64::DecodeError),

    #[error("bincode: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("Jupiter API: {0}")]
    JupiterApi(String),

    #[error("serde_json: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("parse SwapMode: Invalid value `{value}`")]
    ParseSwapMode { value: String },
}

#[derive(Clone, Debug)]
pub struct Price {
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub price: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct PriceResponse {
    data: HashMap<String, PriceData>,
    time_taken: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(dead_code)]
struct PriceData {
    #[serde(with = "field_as_string")]
    id: Pubkey,
    #[serde(rename = "type")]
    price_type: String,
    #[serde(with = "field_as_string")]
    price: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub in_amount: u64,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub out_amount: u64,
    #[serde(with = "field_as_string")]
    pub other_amount_threshold: u64,
    pub swap_mode: String,
    pub slippage_bps: u64,
    pub platform_fee: Option<PlatformFee>,
    #[serde(with = "field_as_string")]
    pub price_impact_pct: f64,
    pub route_plan: Vec<RoutePlan>,
    pub context_slot: Option<u64>,
    pub time_taken: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformFee {
    #[serde(with = "field_as_string")]
    pub amount: u64,
    pub fee_bps: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlan {
    pub swap_info: SwapInfo,
    pub percent: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapInfo {
    #[serde(with = "field_as_string")]
    pub amm_key: Pubkey,
    pub label: Option<String>,
    #[serde(with = "field_as_string")]
    pub input_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub output_mint: Pubkey,
    #[serde(with = "field_as_string")]
    pub in_amount: u64,
    #[serde(with = "field_as_string")]
    pub out_amount: u64,
    #[serde(with = "field_as_string")]
    pub fee_amount: u64,
    #[serde(with = "field_as_string")]
    pub fee_mint: Pubkey,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeInfo {
    #[serde(with = "field_as_string")]
    pub amount: u64,
    #[serde(with = "field_as_string")]
    pub mint: Pubkey,
    pub pct: f64,
}

/// Partially signed transactions required to execute a swap
#[derive(Clone, Debug)]
pub struct Swap {
    pub swap_transaction: VersionedTransaction,
    pub last_valid_block_height: u64,
}

/// Swap instructions
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapInstructions {
    #[serde(with = "field_instruction::option_instruction")]
    pub token_ledger_instruction: Option<Instruction>,
    #[serde(with = "field_instruction::vec_instruction")]
    pub compute_budget_instructions: Vec<Instruction>,
    #[serde(with = "field_instruction::vec_instruction")]
    pub setup_instructions: Vec<Instruction>,
    #[serde(with = "field_instruction::instruction")]
    pub swap_instruction: Instruction,
    #[serde(with = "field_instruction::option_instruction")]
    pub cleanup_instruction: Option<Instruction>,
    #[serde(with = "field_pubkey::vec")]
    pub address_lookup_table_addresses: Vec<Pubkey>,
    pub prioritization_fee_lamports: u64,
}

/// Hashmap, which key is the program id and value is the label. This is used to help map error from transaction by identifying the fault program id. With that, we can use the excludeDexes or dexes parameter.
pub type DexProgramIdToLabel = HashMap<Pubkey, String>;

fn maybe_jupiter_api_error<T>(value: serde_json::Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    #[derive(Deserialize)]
    struct ErrorResponse {
        error: String,
    }
    if let Ok(ErrorResponse { error }) = serde_json::from_value::<ErrorResponse>(value.clone()) {
        Err(Error::JupiterApi(error))
    } else {
        serde_json::from_value(value).map_err(|err| err.into())
    }
}

/// Get simple price for a given input mint, output mint, and amount
pub async fn price(input_mint: Pubkey, output_mint: Pubkey, ui_amount: f64) -> Result<Price> {
    let url = format!(
        "{base_url}?ids={input_mint}&vsToken={output_mint}&amount={ui_amount}",
        base_url = price_api_url(),
    );

    let response: PriceResponse = maybe_jupiter_api_error(reqwest::get(url).await?.json().await?)?;

    let input_data = response.data.get(&input_mint.to_string()).ok_or_else(|| {
        Error::JupiterApi(format!(
            "Input mint {} not found in response data",
            input_mint
        ))
    })?;

    // Calculate the amount to pay
    let price = ui_amount * input_data.price;

    Ok(Price {
        input_mint,
        output_mint,
        price,
    })
}

#[derive(Serialize, Deserialize, Default, PartialEq, Clone, Debug)]
pub enum SwapMode {
    #[default]
    ExactIn,
    ExactOut,
}

impl FromStr for SwapMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "ExactIn" => Ok(Self::ExactIn),
            "ExactOut" => Ok(Self::ExactOut),
            _ => Err(Error::ParseSwapMode { value: s.into() }),
        }
    }
}

impl fmt::Display for SwapMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::ExactIn => write!(f, "ExactIn"),
            Self::ExactOut => write!(f, "ExactOut"),
        }
    }
}

#[derive(Default)]
pub struct QuoteConfig {
    pub slippage_bps: Option<u64>,
    pub swap_mode: Option<SwapMode>,
    pub dexes: Option<Vec<String>>,
    pub exclude_dexes: Option<Vec<String>>,
    pub restrict_intermediate_tokens: Option<bool>,
    pub only_direct_routes: bool,
    pub as_legacy_transaction: Option<bool>,
    pub platform_fee_bps: Option<u64>,
    pub max_accounts: Option<u64>,
    pub auto_slippage: Option<bool>,
    pub max_auto_slippage_bps: Option<u64>,
    pub auto_slippage_collision_usd_value: Option<u64>,
}

/// Get quote for a given input mint, output mint, and amount
pub async fn quote(
    input_mint: Pubkey,
    output_mint: Pubkey,
    amount: u64,
    quote_config: QuoteConfig,
) -> Result<Quote> {
    let url = format!(
        "{base_url}/quote?inputMint={input_mint}&outputMint={output_mint}&amount={amount}&onlyDirectRoutes={}&{}{}{}{}{}{}{}{}{}{}{}",
        quote_config.only_direct_routes,
        quote_config
            .as_legacy_transaction
            .map(|as_legacy_transaction| format!("&asLegacyTransaction={as_legacy_transaction}"))
            .unwrap_or_default(),
        quote_config
            .swap_mode
            .map(|swap_mode| format!("&swapMode={swap_mode}"))
            .unwrap_or_default(),
        quote_config
            .slippage_bps
            .map(|slippage_bps| format!("&slippageBps={slippage_bps}"))
            .unwrap_or_default(),
        quote_config
            .platform_fee_bps
            .map(|platform_fee_bps| format!("&feeBps={platform_fee_bps}"))
            .unwrap_or_default(),
        quote_config
            .dexes
            .map(|dexes| format!("&dexes={}", dexes.into_iter().join(",")))
            .unwrap_or_default(),
        quote_config
            .exclude_dexes
            .map(|exclude_dexes| format!("&excludeDexes={}", exclude_dexes.into_iter().join(",")))
            .unwrap_or_default(),
        quote_config
            .restrict_intermediate_tokens
            .map(|restrict_intermediate_tokens| format!("&restrictIntermediateTokens={restrict_intermediate_tokens}"))
            .unwrap_or_default(),
        quote_config
            .max_accounts
            .map(|max_accounts| format!("&maxAccounts={max_accounts}"))
            .unwrap_or_default(),
        quote_config
            .auto_slippage
            .map(|auto_slippage| format!("&autoSlippage={auto_slippage}"))
            .unwrap_or_default(),
        quote_config
            .max_auto_slippage_bps
            .map(|max_auto_slippage_bps| format!("&maxAutoSlippageBps={max_auto_slippage_bps}"))
            .unwrap_or_default(),
        quote_config
            .auto_slippage_collision_usd_value
            .map(|auto_slippage_collision_usd_value| format!("&autoSlippageCollisionUsdValue={auto_slippage_collision_usd_value}"))
            .unwrap_or_default(),
        base_url=quote_api_url(),
    );

    maybe_jupiter_api_error(reqwest::get(url).await?.json().await?)
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PrioritizationFeeLamports {
    /// Automatically sets a priority fee, capped at 5,000,000 lamports.
    Auto,
    /// Sets an exact amount of lamports for the priority fee.
    Exact { lamports: u64 },
    /// Uses an automatic multiplier for the aut priority fee * this amount.
    AutoMultiplier { multiplier: u64 },
    /// Includes a tip instruction to Jito with the specified lamports.
    JitoTipLamports { lamports: u64 },
    /// Suggests a priority fee based on a level with a maximum cap in lamports.
    PriorityLevelWithMaxLamports {
        priority_level: PriorityLevel,
        max_lamports: u64,
    },
}

/// Represents the priority levels for the `PriorityLevelWithMaxLamports` variant.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PriorityLevel {
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(non_snake_case)]
pub struct SwapRequest {
    #[serde(with = "field_as_string")]
    pub user_public_key: Pubkey,
    pub wrap_and_unwrap_sol: Option<bool>,
    pub use_shared_accounts: Option<bool>,
    #[serde(with = "field_pubkey::option")]
    pub fee_account: Option<Pubkey>,
    #[serde(with = "field_pubkey::option")]
    pub tracking_account: Option<Pubkey>,
    #[deprecated = "please use SwapRequest::prioritization_fee_lamports instead"]
    pub compute_unit_price_micro_lamports: Option<u64>,
    #[serde(with = "field_prioritization_fee")]
    pub prioritization_fee_lamports: PrioritizationFeeLamports,
    pub as_legacy_transaction: Option<bool>,
    pub use_token_ledger: Option<bool>,
    #[serde(with = "field_pubkey::option")]
    pub destination_token_account: Option<Pubkey>,
    pub dynamic_compute_unit_limit: Option<bool>,
    pub skip_user_accounts_rpc_calls: Option<bool>,
    pub dynamic_slippage: Option<DynamicSlippage>,
    pub quote_response: Quote,
}

impl SwapRequest {
    /// Creates new SwapRequest with the given and default values
    pub fn new(user_public_key: Pubkey, quote_response: Quote) -> Self {
        #[allow(deprecated)]
        SwapRequest {
            user_public_key,
            wrap_and_unwrap_sol: Some(true),
            use_shared_accounts: Some(true),
            fee_account: None,
            tracking_account: None,
            compute_unit_price_micro_lamports: None,
            prioritization_fee_lamports: PrioritizationFeeLamports::Auto,
            as_legacy_transaction: Some(false),
            use_token_ledger: Some(false),
            destination_token_account: None,
            dynamic_compute_unit_limit: Some(false),
            skip_user_accounts_rpc_calls: Some(false),
            dynamic_slippage: None,
            quote_response,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicSlippage {
    pub min_bps: u64,
    pub max_bps: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwapResponse {
    pub swap_transaction: String,
    pub last_valid_block_height: u64,
}

/// Get swap serialized transactions for a quote
pub async fn swap(swap_request: SwapRequest) -> Result<Swap> {
    let url = format!("{}/swap", quote_api_url());

    let response = maybe_jupiter_api_error::<SwapResponse>(
        reqwest::Client::builder()
            .build()?
            .post(url)
            .header("Accept", "application/json")
            .json(&swap_request)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?,
    )?;

    fn decode(base64_transaction: String) -> Result<VersionedTransaction> {
        bincode::deserialize(&BASE64_STANDARD.decode(base64_transaction)?).map_err(|err| err.into())
    }

    Ok(Swap {
        swap_transaction: decode(response.swap_transaction)?,
        last_valid_block_height: response.last_valid_block_height,
    })
}

/// Get swap serialized transaction instructions for a quote
pub async fn swap_instructions(swap_request: SwapRequest) -> Result<SwapInstructions> {
    let url = format!("{}/swap-instructions", quote_api_url());

    let response = reqwest::Client::builder()
        .build()?
        .post(url)
        .header("Accept", "application/json")
        .json(&swap_request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(Error::JupiterApi(response.text().await?));
    }

    Ok(response.json::<SwapInstructions>().await?)
}

/// Get a hashmap, which key is the program id and value is the label. This is used to help map error from transaction by identifying the fault program id. With that, we can use the excludeDexes or dexes parameter.
pub async fn program_id_to_label() -> Result<DexProgramIdToLabel> {
    let url = format!("{}/program-id-to-label", quote_api_url());

    let response = reqwest::get(url).await?;

    if !response.status().is_success() {
        return Err(Error::JupiterApi(response.text().await?));
    }

    pub type DexProgramIdToLabelString = HashMap<String, String>;
    let hashmap_string: DexProgramIdToLabelString =
        response.json().await.map_err(Error::Reqwest)?;

    let hashmap_pubkey: DexProgramIdToLabel = hashmap_string
        .into_iter()
        .map(|(key, value)| {
            key.parse::<Pubkey>()
                .map(|pubkey| (pubkey, value))
                .map_err(|e| Error::JupiterApi(format!("Invalid Pubkey in response: {}", e)))
        })
        .collect::<Result<_>>()?;

    Ok(hashmap_pubkey)
}

/// Returns a list of all the tradable mints
pub async fn tokens() -> Result<Vec<Pubkey>> {
    let url = format!("{}/tokens", quote_api_url());

    let response = reqwest::get(url).await?;

    if !response.status().is_success() {
        return Err(Error::JupiterApi(response.text().await?));
    }

    let tokens_string: Vec<String> = response.json().await.map_err(Error::Reqwest)?;

    let tokens: Vec<Pubkey> = tokens_string
        .into_iter()
        .map(|token| {
            token
                .parse::<Pubkey>()
                .map_err(|e| Error::JupiterApi(format!("Invalid Pubkey in response: {}", e)))
        })
        .collect::<Result<_>>()?;

    Ok(tokens)
}
