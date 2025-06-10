use crate::error::{Result, UpgradeError};
use crate::storage::save_new_fw;
use alloc::format;
use botifactory_types::ReleaseBody;
use defmt::debug;
use embedded_nal_async::{Dns, TcpConnect};
use embedded_storage::nor_flash::NorFlash;
use reqwless::client::HttpClient;
use reqwless::request::RequestBuilder;
use semver::Version;

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
        format!(
            "{}/{}/{}/latest",
            self.server_url, self.project_name, self.channel_name
        )
    }

    pub fn previous(self) -> String {
        format!(
            "{}/{}/{}/latest",
            self.server_url, self.project_name, self.channel_name
        )
    }

    pub fn id(self, id: String) -> String {
        format!(
            "{}/{}/{}/{}",
            self.server_url, self.project_name, self.channel_name, id
        )
    }
}

pub struct BotifactoryClient<'a, T, D>
where
    T: TcpConnect + 'a,
    D: Dns + 'a,
{
    url: String,
    client: HttpClient<'a, T, D>,
}

impl<T: embedded_nal_async::TcpConnect, D: embedded_nal_async::Dns> BotifactoryClient<'_, T, D> {
    pub async fn read_version(mut self) -> Result<Version> {
        let mut buffer = [0u8; 4096];
        debug!("building request");
        let headers = [("accept", "application/json")];
        let mut request = self
            .client
            .request(reqwless::request::Method::GET, &self.url)
            .await
            .map_err(UpgradeError::from)?
            .content_type(reqwless::headers::ContentType::ApplicationJson)
            .headers(&headers);

        debug!("sending request");
        let response = request
            .send(&mut buffer)
            .await
            .map_err(UpgradeError::from)?;
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

    pub async fn read_binary<S: NorFlash>(mut self, storage: &mut S) -> Result<()> {
        let mut buffer = [0u8; 4096];
        debug!("building request");
        let headers = [("accept", "application/octet-stream")];

        let mut request = self
            .client
            .request(reqwless::request::Method::GET, &self.url)
            .await
            .map_err(UpgradeError::from)?
            .content_type(reqwless::headers::ContentType::ApplicationOctetStream)
            .headers(&headers);

        debug!("sending request");
        let response = request
            .send(&mut buffer)
            .await
            .map_err(UpgradeError::from)?;
        debug!("status code: {}", response.status);
        if !response.status.is_successful() {
            return Err(UpgradeError::RequestError);
        }

        save_new_fw(storage, response.body().reader()).await
    }
}
