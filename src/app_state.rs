use crate::consts::{ML_SERVER_URL, YRAL_METADATA_URL};
use crate::{canister::individual_user_template::IndividualUserTemplate, config::AppConfig};
use anyhow::{anyhow, Context, Result};
use candid::Principal;
use ic_agent::Agent;
use std::env;
use yral_metadata_client::MetadataClient;
use yup_oauth2::ServiceAccountAuthenticator;

#[derive(Clone)]
pub struct AppState {
    pub agent: ic_agent::Agent,
    pub yral_metadata_client: MetadataClient<true>,
    pub ml_server_grpc_channel: tonic::transport::Channel,
}

impl AppState {
    pub async fn new(app_config: AppConfig) -> Self {
        AppState {
            yral_metadata_client: init_yral_metadata_client(&app_config),
            agent: init_agent().await,
            ml_server_grpc_channel: init_ml_server_grpc_channel().await,
        }
    }

    pub async fn get_individual_canister_by_user_principal(
        &self,
        user_principal: Principal,
    ) -> Result<Principal> {
        let meta = self
            .yral_metadata_client
            .get_user_metadata(user_principal)
            .await
            .context("Failed to get user_metadata from yral_metadata_client")?;

        match meta {
            Some(meta) => Ok(meta.user_canister_id),
            None => Err(anyhow!(
                "user metadata does not exist in yral_metadata_service"
            )),
        }
    }

    pub fn individual_user(&self, user_canister: Principal) -> IndividualUserTemplate<'_> {
        IndividualUserTemplate(user_canister, &self.agent)
    }
}

pub fn init_yral_metadata_client(conf: &AppConfig) -> MetadataClient<true> {
    MetadataClient::with_base_url(YRAL_METADATA_URL.clone())
        .with_jwt_token(conf.yral_metadata_token.clone())
}

pub async fn init_ml_server_grpc_channel() -> tonic::transport::Channel {
    tonic::transport::Channel::from_static(ML_SERVER_URL)
        .connect()
        .await
        .expect("Failed to connect to ML server")
}

pub async fn init_google_sa_key_access_token(conf: &AppConfig) -> String {
    let sa_key_file = conf.google_sa_key.clone();

    // Load service account key
    let sa_key = yup_oauth2::parse_service_account_key(sa_key_file).expect("GOOGLE_SA_KEY");

    let auth = ServiceAccountAuthenticator::builder(sa_key)
        .build()
        .await
        .unwrap();

    let scopes = &["https://www.googleapis.com/auth/bigquery.insertdata"];
    let token = auth.token(scopes).await.unwrap();

    match token.token() {
        Some(t) => t.to_string(),
        _ => panic!("No access token found"),
    }
}

pub async fn init_agent() -> Agent {
    let pk = env::var("RECLAIM_CANISTER_PEM").expect("$RECLAIM_CANISTER_PEM is not set");

    let identity = match ic_agent::identity::BasicIdentity::from_pem(
        stringreader::StringReader::new(pk.as_str()),
    ) {
        Ok(identity) => identity,
        Err(err) => {
            panic!("Unable to create identity, error: {:?}", err);
        }
    };

    // let identity = match ic_agent::identity::Secp256k1Identity::from_pem_file(
    //     "/Users/komalsai/Downloads/generated-id.pem",
    // ) {
    //     Ok(identity) => identity,
    //     Err(err) => {
    //         panic!("Unable to create identity, error: {:?}", err);
    //     }
    // };

    let agent = match Agent::builder()
        .with_url("https://a4gq6-oaaaa-aaaab-qaa4q-cai.raw.ic0.app/") // https://a4gq6-oaaaa-aaaab-qaa4q-cai.raw.ic0.app/
        .with_identity(identity)
        .build()
    {
        Ok(agent) => agent,
        Err(err) => {
            panic!("Unable to create agent, error: {:?}", err);
        }
    };

    // ‼️‼️comment below line in mainnet‼️‼️
    // agent.fetch_root_key().await.unwrap();

    agent
}
