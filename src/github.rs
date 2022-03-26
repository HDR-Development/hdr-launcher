use minreq::Method;
use semver::Version;
use serde_json::Value;
use minreq::Error;

pub type MinResult<T> = Result<T, Error>;

#[derive(Debug)]
pub struct GithubAsset {
    pub name: String,
    pub api_url: String,
    pub size: usize,
}

#[derive(Debug)]
pub struct GithubRelease {
    pub version: Version,
    pub assets: Vec<GithubAsset>,
    pub body: String
}

pub fn make_req_with_headers(
    url: &str,
    is_json: bool,
    client: &str,
    token: Option<&str>
) -> minreq::Request
{
    let accept = if is_json {
        "application/vnd.github.v3+json"
    } else {
        "application/octet-stream"
    };

    let request = minreq::Request::new(Method::Get, url)
        .with_header("Accept", accept)
        .with_header("User-Agent", client);

    if let Some(token) = token {
        request.with_header("Authorization", format!("token {}", token).as_str())
    } else {
        request
    }
}

pub fn value_to_release(value: &Value) -> GithubRelease {
    let version_string = value["tag_name"].as_str().unwrap().trim_start_matches("v");
    let assets = value["assets"].as_array().unwrap();
    let assets = assets.iter().map(|x| {
        GithubAsset {
            name: x["name"].as_str().unwrap().to_string(),
            api_url: x["url"].as_str().unwrap().to_string(),
            size: x["size"].as_u64().unwrap() as usize,
        }
    }).collect();
    let body = value["body"].as_str().unwrap();
    GithubRelease {
        version: Version::parse(version_string).unwrap(),
        assets,
        body: body.to_string()
    }
}

pub fn get_all_releases_for_repository(owner: &str, repo: &str) -> MinResult<Vec<GithubRelease>> {
    let url = format!("https://api.github.com/repos/{}/{}/releases", owner, repo);
    let req = make_req_with_headers(url.as_str(), true, "HDR-Launcher", None);
    let response = req.send()?;
    let releases: Vec<Value> = serde_json::from_str(response.as_str().unwrap())
        .map_err(|_| Error::Other("Failed to parse GitHub assets JSON response!"))?;
    Ok(releases.into_iter().map(|x| value_to_release(&x)).collect())
}

pub fn download_binary_file_with_callback(
    url: &str,
    block_size: usize,
    mut func: impl FnMut(&[u8])
) -> MinResult<()>
{
    let req = make_req_with_headers(url, false, "HDR-Launcher", None);
    let response = req.send_lazy()?;

    let mut bytes = Vec::with_capacity(block_size);
    for (byte_no, byte) in response.into_iter().enumerate() {
        let byte = byte?.0;
        bytes.push(byte);
        if byte_no != 0 && byte_no % block_size == 0 {
            func(bytes.as_slice());
            bytes.clear();
        }
    }
    func(bytes.as_slice());
    bytes.clear();

    Ok(())
}