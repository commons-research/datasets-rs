use futures::TryStreamExt;
use hf_hub::HFClient;

/// Structure for Hugging Face Client
pub struct HuggingFaceSource {
    client: HFClient
}
