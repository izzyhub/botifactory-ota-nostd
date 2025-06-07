use botifactory_types::ReleaseBody;
use crate::error::{UpgradeError, Result};
use reqwless::client::{HttpClient, TlsConfig};
use reqwless::request::RequestBuilder;
use embedded_storage::nor_flash::NorFlash;
use esp_partition_table::PartitionEntry;
use alloc::format;
use defmt::{debug, error, info, warn};
use semver::Version;
use embedded_nal_async::{TcpConnect, Dns};

use alloc::string::String;

pub struct BotifactoryUrlBuilder {
    pub server_url: String,
    pub project_name: String,
    pub channel_name: String,
}

impl BotifactoryUrlBuilder {
    pub fn new(server_url: String, project_name: String, channel_name: String) -> Self {
        Self {
            server_url,
            project_name,
            channel_name,
        }
    }
    pub fn latest(self) -> String {
        format!("{}/{}/{}/latest",
            self.server_url, self.project_name, self.channel_name
        )
    }

    pub fn previous(self) -> String {
        format!("{}/{}/{}/latest",
            self.server_url, self.project_name, self.channel_name
        )
    }

    pub fn id(self, id: String) -> String {
        format!("{}/{}/{}/{}",
            self.server_url, self.project_name, self.channel_name, id
        )
    }
}


pub struct BotifactoryClient<'a, T, D> 
    where
      T: TcpConnect + 'a,
      D: Dns + 'a
{
    url: String,
    client: HttpClient<'a, T, D>,
}

impl<T: embedded_nal_async::TcpConnect, D: embedded_nal_async::Dns> BotifactoryClient<'_, T, D> {
    pub async fn read_version(mut self) -> Result<Version> {
        let mut buffer = [0u8; 4096];
        debug!("building request");
        let headers = [("accept", "application/json")];
        let mut request = self.client
            .request(reqwless::request::Method::GET, &self.url)
            .await
            .map_err(UpgradeError::from)?
            .content_type(reqwless::headers::ContentType::ApplicationJson)
            .headers(&headers);

        debug!("sending request");
        let response = request.send(&mut buffer).await.map_err(UpgradeError::from)?;
        debug!("status code: {}", response.status);
        if !response.status.is_successful() {
            return Err(UpgradeError::RequestError);
        }
        debug!("reading response");
        let response_body = response
            .body()
            .read_to_end()
            .await
            .map_err(UpgradeError::from)?;
        debug!("response read");

        let content = core::str::from_utf8(response_body)?;
        debug!("content read");

        let (release_response, _size): (ReleaseBody, usize) =
            serde_json_core::from_str(content).map_err(UpgradeError::from)?;

        Ok(release_response.release.version)
    }

    /*
    pub async fn read_binary<S: NorFlash>(self, 
        storage: &mut S) -> Result<PartitionEntry> {
            let mut buffer = [0u8; 4096];
            debug!("building request");
            let headers = [("accept", "application/octet-stream")];

            let mut request = self.client
                .request(reqwless::request::Method::GET, &self.url)
                .await
                .map_err(UpgradeError::from)?
                .content_type(reqwless::headers::ContentType::ApplicationOctetStream)
                .headers(&headers);

            debug!("sending request");
            let response = request.send(&mut buffer).await.map_err(UpgradeError::from)?;
        debug!("status code: {}", response.status);
        if !response.status.is_successful() {
            Err(UpgradeError::RequestError);
        }
        debug!("reading response");
        let response_body = response
            .body()
            .read_to_end()
            .await
            .map_err(UpgradeError::from)?;
        debug!("response read");

        let content = core::str::from_utf8(response_body)?;
        debug!("content read");

        let (release_response, _size): (ReleaseBody, usize) =
            serde_json_core::from_str(content).map_err(UpgradeError::from)?;

        Ok(release_response.release.version)
    }
    */
}
