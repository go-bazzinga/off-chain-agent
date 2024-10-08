use std::sync::Arc;

use crate::{
    app_state::AppState,
    canister::mlfeed_cache::off_chain::{Empty, UpdateMlFeedCacheRequest},
};
use candid::Principal;
use off_chain::{off_chain_canister_server::OffChainCanister, MlFeedCacheItem};
use yral_canisters_client::individual_user_template::Result23;

pub mod off_chain {
    tonic::include_proto!("offchain_canister");
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("offchain_canister_descriptor");
}

pub struct OffChainCanisterService {
    pub shared_state: Arc<AppState>,
}

#[tonic::async_trait]
impl OffChainCanister for OffChainCanisterService {
    async fn update_ml_feed_cache(
        &self,
        request: tonic::Request<UpdateMlFeedCacheRequest>,
    ) -> core::result::Result<tonic::Response<Empty>, tonic::Status> {
        let request = request.into_inner();

        let state = self.shared_state.clone();

        let canister_principal = Principal::from_text(request.user_canister_id)
            .map_err(|_| tonic::Status::invalid_argument("Invalid canister principal"))?;
        let user_canister = state.individual_user(canister_principal);

        let arg0 = request.items.into_iter().map(|x| x.into()).collect();

        let res = user_canister
            .update_ml_feed_cache(arg0)
            .await
            .map_err(|e| {
                tonic::Status::internal(format!("Error updating ml feed cache: {:?}", e))
            })?;

        if let Result23::Err(err) = res {
            log::error!("Error updating ml feed cache: {:?}", err);
            return Err(tonic::Status::internal(format!(
                "Error updating ml feed cache: {:?}",
                err
            )));
        }

        Ok(tonic::Response::new(Empty {}))
    }
}

impl From<MlFeedCacheItem> for yral_canisters_client::individual_user_template::MlFeedCacheItem {
    fn from(item: MlFeedCacheItem) -> Self {
        yral_canisters_client::individual_user_template::MlFeedCacheItem {
            post_id: item.post_id,
            canister_id: Principal::from_text(item.canister_id).unwrap(),
            video_id: item.video_id,
            creator_principal_id: if item.creator_principal_id.is_empty() {
                None
            } else {
                Some(Principal::from_text(item.creator_principal_id).unwrap())
            },
        }
    }
}
