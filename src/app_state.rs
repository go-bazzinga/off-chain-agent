use crate::config::AppConfig;
use crate::consts::YRAL_METADATA_URL;
use crate::qstash::QStashState;
use anyhow::{anyhow, Context, Result};
use candid::Principal;
use firestore::{FirestoreDb, FirestoreDbOptions};
use google_cloud_bigquery::client::{Client, ClientConfig};
use hyper_util::client::legacy::connect::HttpConnector;
use ic_agent::Agent;
use s3::creds::Credentials;
use s3::{Bucket, Region};
use std::env;
use yral_canisters_client::individual_user_template::IndividualUserTemplate;
use yral_metadata_client::MetadataClient;
use yup_oauth2::hyper_rustls::HttpsConnector;
use yup_oauth2::{authenticator::Authenticator, ServiceAccountAuthenticator};

#[derive(Clone)]
pub struct AppState {
    pub agent: ic_agent::Agent,
    pub yral_metadata_client: MetadataClient<true>,
    pub auth: Authenticator<HttpsConnector<HttpConnector>>,
    pub firestoredb: FirestoreDb,
    pub qstash: QStashState,
    pub bigquery_client: Client,
}

impl AppState {
    pub async fn new(app_config: AppConfig) -> Self {
        AppState {
            yral_metadata_client: init_yral_metadata_client(&app_config),
            agent: init_agent().await,
            auth: init_auth().await,
            // ml_server_grpc_channel: init_ml_server_grpc_channel().await,
            firestoredb: init_firestoredb().await,
            qstash: init_qstash(),
            bigquery_client: init_bigquery_client().await,
        }
    }

    pub async fn get_access_token(&self, scopes: &[&str]) -> String {
        let auth = &self.auth;
        let token = auth.token(scopes).await.unwrap();

        match token.token() {
            Some(t) => t.to_string(),
            _ => panic!("No access token found"),
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

pub async fn init_agent() -> Agent {
    #[cfg(not(feature = "local-bin"))]
    {
        let pk = env::var("RECLAIM_CANISTER_PEM").expect("$RECLAIM_CANISTER_PEM is not set");

        let identity = match ic_agent::identity::BasicIdentity::from_pem(
            stringreader::StringReader::new(pk.as_str()),
        ) {
            Ok(identity) => identity,
            Err(err) => {
                panic!("Unable to create identity, error: {:?}", err);
            }
        };

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

        agent
    }

    #[cfg(feature = "local-bin")]
    {
        let agent = Agent::builder()
            .with_url("https://ic0.app")
            .build()
            .unwrap();

        // agent.fetch_root_key().await.unwrap();

        agent
    }
}

pub async fn init_auth() -> Authenticator<HttpsConnector<HttpConnector>> {
    let sa_key_file = env::var("GOOGLE_SA_KEY").expect("GOOGLE_SA_KEY is required");

    // Load your service account key
    let sa_key = yup_oauth2::parse_service_account_key(sa_key_file).expect("GOOGLE_SA_KEY.json");

    ServiceAccountAuthenticator::builder(sa_key)
        .build()
        .await
        .unwrap()
}

pub async fn init_firestoredb() -> FirestoreDb {
    let options = FirestoreDbOptions::new("hot-or-not-feed-intelligence".to_string())
        .with_database_id("ic-pump-fun".to_string());

    FirestoreDb::with_options(options)
        .await
        .expect("failed to create firestore db")
}

pub fn init_qstash() -> QStashState {
    let qstash_key =
        env::var("QSTASH_CURRENT_SIGNING_KEY").expect("QSTASH_CURRENT_SIGNING_KEY is required");

    QStashState::init(qstash_key)
}

pub async fn init_bigquery_client() -> Client {
    let (config, _) = ClientConfig::new_with_auth().await.unwrap();
    Client::new(config).await.unwrap()
}
