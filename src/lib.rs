use std::{fmt, str::FromStr};

use solana_sdk::transaction::VersionedTransaction;

use {
    serde::{Deserialize, Serialize},
    solana_sdk::pubkey::{ParsePubkeyError, Pubkey},
    std::collections::HashMap,
};

mod field_as_string;

/// A `Result` alias where the `Err` case is `jup_ag::Error`.
pub type Result<T> = std::result::Result<T, Error>;

const QUOTE_API_URL: &str = "https://quote-api.jup.ag/v6"; // Reference: https://quote-api.jup.ag/v4/docs/static/index.html
const PRICE_API_URL: &str = "https://price.jup.ag/v1"; // Reference: https://quote-api.jup.ag/docs/static/index.html

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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    #[serde(with = "field_as_string", rename = "id")]
    pub input_mint: Pubkey,
    #[serde(rename = "mintSymbol")]
    pub input_symbol: String,
    #[serde(with = "field_as_string", rename = "vsToken")]
    pub output_mint: Pubkey,
    #[serde(rename = "vsTokenSymbol")]
    pub output_symbol: String,
    pub price: f64,
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
    pub percent: f64,
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
}

/// Hashmap of possible swap routes from input mint to an array of output mints
pub type RouteMap = HashMap<Pubkey, Vec<Pubkey>>;

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

/// Get simple price for a given input mint, output mint and amount
pub async fn price(
    input_mint: Pubkey,
    output_mint: Pubkey,
    ui_amount: f64,
) -> Result<Price> {
    let url =
        format!("{PRICE_API_URL}/price?id={input_mint}&vsToken={output_mint}&amount={ui_amount}");
    maybe_jupiter_api_error(reqwest::get(url).await?.json().await?)
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
    pub dexes: Option<Vec<Pubkey>>,
    pub exclude_dexes: Option<Vec<Pubkey>>,
    pub only_direct_routes: bool,
    pub as_legacy_transaction: Option<bool>,
    pub platform_fee_bps: Option<u64>,
    pub max_accounts: Option<u64>,
}

/// Get quote for a given input mint, output mint and amount
pub async fn quote(
    input_mint: Pubkey,
    output_mint: Pubkey,
    amount: u64,
    quote_config: QuoteConfig,
) -> Result<Quote> {
    let url = format!(
        "{QUOTE_API_URL}/quote?inputMint={input_mint}&outputMint={output_mint}&amount={amount}&onlyDirectRoutes={}&{}{}{}{}",
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
            .fee_bps
            .map(|fee_bps| format!("&feeBps={platform_fee_bps}"))
            .unwrap_or_default(),
        quote_config
            .dexes
            .map(|dexes| format!("&dexes={dexes}"))
            .unwrap_or_default(),
        quote_config
            .exclude_dexes
            .map(|exclude_dexes| format!("&excludeDexes={exclude_dexes}"))
            .unwrap_or_default(),
        quote_config
            .max_accounts
            .map(|max_accounts| format!("&maxAccounts={max_accounts}"))
            .unwrap_or_default(),
    );

    maybe_jupiter_api_error(reqwest::get(url).await?.json().await?)
}

#[derive(Default)]
pub struct SwapConfig {
    pub wrap_unwrap_sol: Option<bool>,
    pub fee_account: Option<Pubkey>,
    pub compute_unit_price_micro_lamports: Option<usize>,
    pub as_legacy_transaction: Option<bool>,
}

/// Get swap serialized transactions for a quote
pub async fn swap_with_config(
    route: Quote,
    user_public_key: Pubkey,
    swap_config: SwapConfig,
) -> Result<Swap> {
    let url = format!("{QUOTE_API_URL}/swap");

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    #[allow(non_snake_case)]
    struct SwapRequest {
        route: Quote,
        wrap_unwrap_SOL: Option<bool>,
        fee_account: Option<String>,
        #[serde(with = "field_as_string")]
        user_public_key: Pubkey,
        as_legacy_transaction: Option<bool>,
        compute_unit_price_micro_lamports: Option<usize>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SwapResponse {
        swap_transaction: String,
    }

    let request = SwapRequest {
        route,
        wrap_unwrap_SOL: swap_config.wrap_unwrap_sol,
        user_public_key,
        as_legacy_transaction: swap_config.as_legacy_transaction,
        compute_unit_price_micro_lamports: swap_config.compute_unit_price_micro_lamports,
        fee_account: None,
    };

    let response = maybe_jupiter_api_error::<SwapResponse>(
        reqwest::Client::builder()
            .build()?
            .post(url)
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?,
    )?;

    fn decode(base64_transaction: String) -> Result<VersionedTransaction> {
        bincode::deserialize(&base64::decode(base64_transaction)?).map_err(|err| err.into())
    }

    Ok(Swap {
        swap_transaction: decode(response.swap_transaction)?,
    })
}

/// Get swap serialized transactions for a quote using `SwapConfig` defaults
pub async fn swap(route: Quote, user_public_key: Pubkey) -> Result<Swap> {
    swap_with_config(route, user_public_key, SwapConfig::default()).await
}

/// Returns a hash map, input mint as key and an array of valid output mint as values
pub async fn route_map() -> Result<RouteMap> {
    let url = format!("{QUOTE_API_URL}/indexed-route-map?onlyDirectRoutes=false");

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct IndexedRouteMap {
        mint_keys: Vec<String>,
        indexed_route_map: HashMap<usize, Vec<usize>>,
    }

    let response = reqwest::get(url).await?.json::<IndexedRouteMap>().await?;

    let mint_keys = response
        .mint_keys
        .into_iter()
        .map(|x| x.parse::<Pubkey>().map_err(|err| err.into()))
        .collect::<Result<Vec<Pubkey>>>()?;

    let mut route_map = HashMap::new();
    for (from_index, to_indices) in response.indexed_route_map {
        route_map.insert(
            mint_keys[from_index],
            to_indices.into_iter().map(|i| mint_keys[i]).collect(),
        );
    }

    Ok(route_map)
}
