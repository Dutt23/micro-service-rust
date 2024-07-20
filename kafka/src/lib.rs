pub mod avro;
pub mod consumer;
pub mod producer;
pub mod utils;

pub mod commons {
    use std::env;

    use schema_registry_converter::async_impl::schema_registry::SrSettings;

    pub fn create_schema_registry_settings(schema_registry_url: String) -> SrSettings {
        let schema_api_key =
            env::var("SCHEMA_API_KEY").expect("schema key key not found in variables");
        let schema_api_password =
            env::var("SCHEMA_API_SECRET").expect("schema password key not found in variables");
        SrSettings::new_builder(schema_registry_url)
            .set_basic_authorization(&schema_api_key, Some(&schema_api_password))
            // .build_with(builder)
            .build()
            .expect("msg")
    }
}
