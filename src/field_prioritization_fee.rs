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
        PrioritizationFeeLamports::AutoMultiplier { multiplier } => {
            let value = serde_json::json!({ "autoMultiplier": multiplier });
            value.serialize(serializer)
        }
        PrioritizationFeeLamports::JitoTipLamports { lamports } => {
            let value = serde_json::json!({ "jitoTipLamports": lamports });
            value.serialize(serializer)
        }
        PrioritizationFeeLamports::PriorityLevelWithMaxLamports {
            priority_level,
            max_lamports,
        } => {
            let value = serde_json::json!({
                "priorityLevelWithMaxLamports": {
                    "priorityLevel": priority_level,
                    "maxLamports": max_lamports
                }
            });
            value.serialize(serializer)
        }
    }
}
