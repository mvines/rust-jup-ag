use {crate::PrioritizationFeeLamports, serde::Serialize, serde::Serializer};

pub fn serialize<S>(
    prioritization_fee_lamports: &PrioritizationFeeLamports,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match prioritization_fee_lamports {
        PrioritizationFeeLamports::Auto => "auto".serialize(serializer),
        PrioritizationFeeLamports::Exact { lamports } => lamports.serialize(serializer),
    }
}
