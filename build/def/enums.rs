use crate::def::service::get_services;
use endpoint_gen::model::{EnumVariant, Type};

pub fn get_service_enum() -> Type {
    Type::enum_(
        "service".to_owned(),
        get_services()
            .iter()
            .map(|s| EnumVariant::new(s.name.clone(), s.id as _))
            .collect::<Vec<EnumVariant>>(),
    )
}
pub fn get_enums() -> Vec<Type> {
    vec![
        Type::enum_(
            "role".to_owned(),
            vec![
                EnumVariant::new("guest", 0),
                EnumVariant::new("user", 1),
                EnumVariant::new("trader", 2),
                EnumVariant::new("developer", 3),
                EnumVariant::new("admin", 4),
            ],
        ),
        Type::enum_(
            "block_chain".to_owned(),
            vec![
                EnumVariant::new("EthereumMainnet", 0),
                EnumVariant::new("EthereumGoerli", 1),
                EnumVariant::new("BscMainnet", 2),
                EnumVariant::new("BscTestnet", 3),
                EnumVariant::new("LocalNet", 4),
                EnumVariant::new("EthereumSepolia", 5),
            ],
        ),
        Type::enum_(
            "dex".to_owned(),
            vec![
                EnumVariant::new("UniSwap", 0),
                EnumVariant::new("PancakeSwap", 1),
                EnumVariant::new("SushiSwap", 2),
            ],
        ),
        Type::enum_(
            "dex_path_format".to_owned(),
            vec![
                EnumVariant::new("Json", 0),
                EnumVariant::new("TransactionData", 1),
                EnumVariant::new("TransactionHash", 2),
            ],
        ),
        get_service_enum(),
    ]
}
