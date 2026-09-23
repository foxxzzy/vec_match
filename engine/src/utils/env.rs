use std::env;

pub fn load_env(key: String) -> Result<String, Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let env_var = match env::var(key) {
        Ok(key) => key,
        Err(error) => return Err(Box::new(error)),
    };

    Ok(env_var)
}
