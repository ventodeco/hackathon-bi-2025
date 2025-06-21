use aws_sdk_s3::{
    config::{Credentials, Region},
    Client,
    presigning::PresigningConfig,
    primitives::ByteStream,
};
use std::time::Duration;
use anyhow::Result;

#[derive(Clone)]
pub struct MinioService {
    client: Client,
    bucket_name: String,
}

impl MinioService {
    pub async fn new(endpoint: &str, access_key: &str, secret_key: &str, bucket_name: &str) -> Result<Self> {
        let config = aws_sdk_s3::config::Builder::new()
            .endpoint_url(endpoint)
            .region(Region::new("us-east-1"))
            .credentials_provider(Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "minio",
            ))
            .force_path_style(true)
            .behavior_version_latest()
            .build();

        let client = Client::from_conf(config);

        Ok(Self {
            client,
            bucket_name: bucket_name.to_string(),
        })
    }

    pub async fn generate_presigned_url(&self, file_name: String, expires_in: Duration) -> Result<String> {
        let object_key = format!("{}", file_name);
        let presigned_config = PresigningConfig::builder()
            .expires_in(expires_in)
            .build()?;

        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(object_key)
            .presigned(presigned_config)
            .await?;

        Ok(presigned_request.uri().to_string())
    }

    pub async fn generate_view_url(&self, file_name: String) -> Result<String> {
        self.generate_presigned_url(file_name, Duration::from_secs(3600)).await
    }

    pub async fn generate_upload_url(&self, file_name: String, expires_in: Duration) -> Result<String> {
        let object_key = format!("{}", file_name);
        let presigned_config = PresigningConfig::builder()
            .expires_in(expires_in)
            .build()?;

        let presigned_request = self
            .client
            .put_object()
            .bucket(&self.bucket_name)
            .key(object_key)
            .content_type("image/jpeg")
            .presigned(presigned_config)
            .await?;

        // Log the generated URL for debugging
        println!("Generated presigned URL: {}", presigned_request.uri());

        Ok(presigned_request.uri().to_string())
    }

    pub async fn upload_file(&self, file_name: String, content: Vec<u8>, content_type: Option<String>) -> Result<String> {
        let object_key = format!("{}", file_name);
        let byte_stream = ByteStream::from(content);

        let mut put_object = self
            .client
            .put_object()
            .bucket(&self.bucket_name)
            .key(&object_key)
            .body(byte_stream);

        // Set content type if provided
        if let Some(ct) = content_type {
            put_object = put_object.content_type(ct);
        }

        put_object.send().await?;

        // Generate a view URL for the uploaded file
        let view_url = self.generate_view_url(object_key).await?;
        
        Ok(view_url)
    }

    pub async fn upload_file_with_metadata(
        &self, 
        file_name: String, 
        content: Vec<u8>, 
        content_type: Option<String>,
        metadata: std::collections::HashMap<String, String>
    ) -> Result<String> {
        let object_key = format!("{}", file_name);
        let byte_stream = ByteStream::from(content);

        let mut put_object = self
            .client
            .put_object()
            .bucket(&self.bucket_name)
            .key(&object_key)
            .body(byte_stream);

        // Set content type if provided
        if let Some(ct) = content_type {
            put_object = put_object.content_type(ct);
        }

        // Add metadata
        for (key, value) in metadata {
            put_object = put_object.metadata(key, value);
        }

        put_object.send().await?;

        // Generate a view URL for the uploaded file
        let view_url = self.generate_view_url(object_key).await?;
        
        Ok(view_url)
    }

    pub async fn delete_file(&self, file_name: String) -> Result<()> {
        let object_key = format!("{}", file_name);
        
        self
            .client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(object_key)
            .send()
            .await?;

        Ok(())
    }

    pub async fn file_exists(&self, file_name: String) -> Result<bool> {
        let object_key = format!("{}", file_name);
        
        match self
            .client
            .head_object()
            .bucket(&self.bucket_name)
            .key(object_key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
