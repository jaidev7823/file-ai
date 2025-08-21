use anyhow::{anyhow, Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use hf_hub::api::sync::Api;
use lazy_static::lazy_static;
use std::sync::Mutex;
use tokenizers::Tokenizer;
// Global model instance (loaded once)
lazy_static! {
    static ref MODEL: Mutex<Option<EmbeddingModel>> = Mutex::new(None);
}

struct EmbeddingModel {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl EmbeddingModel {
    async fn new() -> Result<Self> {
        println!("Creating Hugging Face API...");
        let api = Api::new().unwrap();

        println!("API created, fetching files...");
        let repo = api.model("sentence-transformers/all-MiniLM-L6-v2".to_string());

        let tokenizer_filename = repo
            .get("tokenizer.json")
            .with_context(|| "Failed to download tokenizer.json")?;
        println!("Tokenizer path: {}", tokenizer_filename.display());

        // Load the safetensors file
        let weights_filename = repo
            .get("model.safetensors")
            .with_context(|| "Failed to download model.safetensors")?;
        println!("Weights path: {}", weights_filename.display());

        let config_filename = repo
            .get("config.json")
            .with_context(|| "Failed to download config.json")?;
        println!("Config path: {}", config_filename.display());

        let tokenizer = Tokenizer::from_file(&tokenizer_filename).map_err(|e| anyhow!(e))?;
        let config: Config = serde_json::from_slice(&std::fs::read(&config_filename)?)?;
        let device = Device::Cpu;

        // Use from_mmaped_safetensors instead
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_filename], DType::F32, &device)?
        };
        let model = BertModel::load(vb, &config)?;

        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    fn encode(&self, text: &str) -> Result<Vec<f32>> {
        // Tokenize
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenizer error: {}", e))
            .with_context(|| format!("Failed to encode text: {}", text))?;
        let tokens = encoding.get_ids();
        let token_ids = Tensor::new(&tokens[..], &self.device)?.unsqueeze(0)?;
        let token_type_ids = Tensor::zeros(token_ids.dims(), DType::I64, token_ids.device())?;
        // Forward pass
        let embeddings = self.model.forward(&token_ids, &token_type_ids, None)?;
        // Mean pooling (sentence-transformers style)
        let sentence_embedding = embeddings.mean(1)?;

        // Convert to Vec<f32>
        let embedding: Vec<f32> = sentence_embedding.to_vec1()?;

        Ok(embedding)
    }
}

// Initialize model (call once at app startup)
pub fn init_embedding_model() -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let model = EmbeddingModel::new().await?;
        let mut global_model = MODEL.lock().unwrap();
        *global_model = Some(model);
        Ok(())
    })
}

// Keep your normalize function unchanged
pub fn normalize(v: Vec<f32>) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        v
    } else {
        v.iter().map(|x| x / norm).collect()
    }
}

// Updated get_embedding to use anyhow::Result
pub fn get_embedding(text: &str) -> Result<Vec<f32>> {
    let model_guard = MODEL.lock().unwrap();
    let model = model_guard
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Model not initialized"))?;

    let embedding = model.encode(text)?;
    Ok(normalize(embedding))
}

// Updated get_batch_embeddings_with_progress to use anyhow::Result
pub fn get_batch_embeddings_with_progress<F>(
    texts: &[String],
    mut progress_callback: F,
) -> Result<Vec<Vec<f32>>>
where
    F: FnMut(usize, usize),
{
    let model_guard = MODEL.lock().unwrap();
    let model = model_guard
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Model not initialized"))?;

    let mut all_embeddings = Vec::new();
    let total: usize = texts.len();

    for (idx, text) in texts.iter().enumerate() {
        let current = idx + 1;
        progress_callback(current, total);

        let embedding = model.encode(text)?;
        all_embeddings.push(normalize(embedding));
    }

    Ok(all_embeddings)
}
