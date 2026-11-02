use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use billing::ceremony_format::{Batch, PeriodPublicKey, Totals};
use billing::period::{PeriodId, PeriodLength};
use billing_ceremony::{ShareFile, generate, tally};

const USAGE: &str = "\
Offline (touch keys; run on a machine without network):

  billing-ceremony keygen --keys-out <dir> --shares-out <dir>
                          [--period current|next|<id>] [--count <n>]
                          [--period-length month|<n>s] [--threshold 2] [--holders 3]
      Generates period key pairs: <keys-out>/<period>.pub for the server and
      <shares-out>/<period>-holder-<i>.json for each holder. The private key is
      never written anywhere.

  billing-ceremony tally --batch <file> --share <file> --share <file> ... --out <file>
      Recovers the period key from the shares, opens the batch and writes only
      the per-site totals.

Online (admin gateway; token from --token or CR_ADMIN_TOKEN):

  billing-ceremony upload-keys --api <url> --key <file.pub> [--key <file.pub> ...]
  billing-ceremony fetch-batch --api <url> --period <id> --out <file>
  billing-ceremony upload-totals --api <url> --totals <file>

  <url> is the admin gateway, e.g. http://127.0.0.1:3001";

fn main() -> ExitCode {
    run(std::env::args().skip(1).collect())
}

fn run(args: Vec<String>) -> ExitCode {
    let command = args.first().map(String::as_str);
    let result =
        parse(args.get(1..).unwrap_or_default()).and_then(|args| match command {
            Some("keygen") => keygen(args),
            Some("tally") => run_tally(args),
            Some("upload-keys") => upload_keys(args),
            Some("fetch-batch") => fetch_batch(args),
            Some("upload-totals") => upload_totals(args),
            _ => Err(USAGE.to_string()),
        });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug)]
struct Args(HashMap<String, Vec<String>>);

impl Args {
    fn get(&self, name: &str) -> Option<&str> {
        self.0
            .get(name)
            .and_then(|values| values.last())
            .map(String::as_str)
    }

    fn all(&self, name: &str) -> &[String] {
        self.0
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    fn required(&self, name: &str) -> Result<&str, String> {
        self.get(name)
            .ok_or_else(|| format!("missing --{name}\n\n{USAGE}"))
    }

    fn number<T: std::str::FromStr>(&self, name: &str, default: T) -> Result<T, String> {
        self.get(name)
            .map(|value| {
                value
                    .parse()
                    .map_err(|_| format!("--{name}: not a number"))
            })
            .unwrap_or(Ok(default))
    }

    fn period(&self) -> Result<PeriodId, String> {
        self.required("period")?
            .parse()
            .map(PeriodId::new)
            .map_err(|_| "--period: not a number".to_string())
    }
}

fn parse(args: &[String]) -> Result<Args, String> {
    let mut values: HashMap<String, Vec<String>> = HashMap::new();
    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        let name = flag
            .strip_prefix("--")
            .ok_or_else(|| format!("unexpected {flag:?}\n\n{USAGE}"))?;
        let value = iter
            .next()
            .ok_or_else(|| format!("--{name} needs a value"))?;
        values
            .entry(name.to_string())
            .or_default()
            .push(value.clone());
    }
    Ok(Args(values))
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|e| format!("{path:?}: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{path:?}: {e}"))
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| format!("{path:?}: {e}"))
}

fn keygen(args: Args) -> Result<(), String> {
    let keys_out = PathBuf::from(args.required("keys-out")?);
    let shares_out = PathBuf::from(args.required("shares-out")?);
    let length: PeriodLength = args
        .get("period-length")
        .unwrap_or("month")
        .parse()
        .map_err(|e| format!("{e}"))?;
    let first = match args.get("period").unwrap_or("next") {
        "current" => length.current(),
        "next" => length.current().next(),
        _ => args.period()?,
    };
    let count: u64 = args.number("count", 1)?;
    let threshold: u8 = args.number("threshold", 2)?;
    let holders: u8 = args.number("holders", 3)?;

    for dir in [&keys_out, &shares_out] {
        fs::create_dir_all(dir).map_err(|e| format!("{dir:?}: {e}"))?;
    }
    for offset in 0..count {
        let period = PeriodId::new(
            first
                .get()
                .checked_add(offset)
                .ok_or("period index overflow")?,
        );
        // A second key for a period would leave whichever one the server got
        // without matching shares.
        let key_path = keys_out.join(format!("{period}.pub"));
        if key_path.exists() {
            return Err(format!(
                "{key_path:?} exists: period {period} already has a key"
            ));
        }
        let generated =
            generate(period, threshold, holders).map_err(|e| e.to_string())?;
        for share in &generated.shares {
            let path = shares_out.join(format!("{period}-holder-{}.json", share.holder));
            write_json(&path, share)?;
        }
        fs::write(&key_path, generated.public_key.to_base64url())
            .map_err(|e| format!("{key_path:?}: {e}"))?;
        println!(
            "period {period} ({}): key {}, {threshold}-of-{holders} shares",
            length.label(period),
            generated.public_key.fingerprint()
        );
    }
    Ok(())
}

fn run_tally(args: Args) -> Result<(), String> {
    let batch: Batch = read_json(Path::new(args.required("batch")?))?;
    let shares = args
        .all("share")
        .iter()
        .map(|path| read_json::<ShareFile>(Path::new(path)))
        .collect::<Result<Vec<_>, _>>()?;
    let out = PathBuf::from(args.required("out")?);

    let totals = tally(&batch, &shares).map_err(|e| e.to_string())?;

    write_json(&out, &totals)?;
    println!(
        "period {}: {} records, {} dropped, {} sites -> {out:?}",
        totals.period,
        totals.records,
        totals.dropped,
        totals.per_site_millicents.len()
    );
    Ok(())
}

struct Admin {
    base: String,
    authorization: String,
}

impl Admin {
    fn from_args(args: &Args) -> Result<Self, String> {
        let token = args
            .get("token")
            .map(str::to_string)
            .or_else(|| std::env::var("CR_ADMIN_TOKEN").ok())
            .ok_or("missing --token (or CR_ADMIN_TOKEN)")?;
        Ok(Admin {
            base: args
                .required("api")?
                .trim_end_matches('/')
                .to_string(),
            authorization: format!("Bearer {token}"),
        })
    }

    fn request(&self, method: &str, path: &str) -> ureq::Request {
        ureq::request(method, &format!("{}/admin/billing{path}", self.base))
            .set("Authorization", &self.authorization)
    }
}

fn describe(err: ureq::Error) -> String {
    match err {
        ureq::Error::Status(code, response) => format!(
            "admin gateway answered {code} {}",
            response.status_text()
        ),
        ureq::Error::Transport(transport) => {
            format!("admin gateway unreachable: {transport}")
        }
    }
}

fn upload_keys(args: Args) -> Result<(), String> {
    let admin = Admin::from_args(&args)?;
    let keys = args.all("key");
    if keys.is_empty() {
        return Err(format!("missing --key\n\n{USAGE}"));
    }
    for path in keys {
        let path = Path::new(path);
        let period = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(".pub"))
            .and_then(|index| index.parse().ok())
            .map(PeriodId::new)
            .ok_or_else(|| format!("{path:?}: expected <period>.pub"))?;
        let text = fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
        let key = PeriodPublicKey::from_base64url(&text)
            .map_err(|e| format!("{path:?}: {e}"))?;
        admin
            .request("PUT", &format!("/periods/{period}/key"))
            .send_json(serde_json::json!({ "publicKey": key.to_base64url() }))
            .map_err(|e| format!("period {period}: {}", describe(e)))?;
        println!("period {period}: key {} uploaded", key.fingerprint());
    }
    Ok(())
}

fn fetch_batch(args: Args) -> Result<(), String> {
    let admin = Admin::from_args(&args)?;
    let period = args.period()?;
    let out = PathBuf::from(args.required("out")?);
    let batch: Batch = admin
        .request("GET", &format!("/periods/{period}/batch"))
        .call()
        .map_err(describe)?
        .into_json()
        .map_err(|e| e.to_string())?;
    write_json(&out, &batch)?;
    println!(
        "period {period}: {} records -> {out:?}",
        batch.records.len()
    );
    Ok(())
}

fn upload_totals(args: Args) -> Result<(), String> {
    let admin = Admin::from_args(&args)?;
    let totals: Totals = read_json(Path::new(args.required("totals")?))?;
    let settlement: serde_json::Value = admin
        .request("POST", &format!("/periods/{}/totals", totals.period))
        .send_json(&totals)
        .map_err(describe)?
        .into_json()
        .map_err(|e| e.to_string())?;
    println!("period {}: settled {settlement}", totals.period);
    Ok(())
}

// allow: SIZE_OK — unit tests are required to live in this bin's main.rs.
#[cfg(test)]
mod tests {
    use std::io::{BufRead, Read, Write};
    use std::net::TcpListener;
    use std::time::Duration;

    use billing::ceremony_format::{
        Batch, BatchRecord, PeriodPublicKey, SealedUsage, Totals,
    };
    use billing::period::PeriodId;
    use billing_ceremony::{ShareFile, generate, seal_usage};
    use googletest::prelude::*;
    use rstest::{fixture, rstest};
    use tempfile::TempDir;

    use super::*;

    const TEST_TOKEN: &str = "test-admin-token";
    const TEST_PERIOD: PeriodId = PeriodId::new(681);

    #[fixture]
    fn keys() -> billing_ceremony::KeyGeneration {
        generate(TEST_PERIOD, 2, 3).unwrap()
    }

    fn temp_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    fn write_key_file(dir: &TempDir, period: PeriodId, key: &PeriodPublicKey) -> PathBuf {
        let path = dir.path().join(format!("{period}.pub"));
        fs::write(&path, key.to_base64url()).unwrap();
        path
    }

    fn write_share_files(dir: &TempDir, shares: &[ShareFile]) -> Vec<PathBuf> {
        shares
            .iter()
            .map(|share| {
                let path = dir.path().join(format!(
                    "{}-holder-{}.json",
                    share.period, share.holder
                ));
                write_json(&path, share).unwrap();
                path
            })
            .collect()
    }

    fn write_batch_file(dir: &TempDir, batch: &Batch) -> PathBuf {
        let path = dir.path().join("batch.json");
        write_json(&path, batch).unwrap();
        path
    }

    fn batch_for(key: &PeriodPublicKey, records: &[(u64, &str)]) -> Batch {
        Batch {
            period: TEST_PERIOD,
            records: records
                .iter()
                .map(|(weight, site)| {
                    BatchRecord::new(*weight, &seal_usage(key, TEST_PERIOD, site))
                })
                .collect(),
        }
    }

    #[derive(Debug)]
    struct RecordedRequest {
        method: String,
        path: String,
        authorization: String,
        body: String,
    }

    fn fake_admin_server<F>(
        handler: F,
        expected_requests: usize,
    ) -> (String, std::thread::JoinHandle<Vec<RecordedRequest>>)
    where
        F: Fn(&RecordedRequest) -> (u16, &'static str) + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handler = std::sync::Arc::new(handler);
        let handle = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for _ in 0..expected_requests {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
                let mut writer = stream;

                let mut first_line = String::new();
                reader.read_line(&mut first_line).unwrap();
                let mut parts = first_line.split_whitespace();
                let method = parts.next().unwrap().to_string();
                let path = parts.next().unwrap().to_string();

                let mut authorization = String::new();
                let mut content_length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.strip_prefix("Authorization:") {
                        authorization = value.trim().to_string();
                    }
                    if let Some(value) = line.strip_prefix("Content-Length:") {
                        content_length = value.trim().parse().unwrap_or(0);
                    }
                }

                let mut body = vec![0; content_length];
                if content_length > 0 {
                    reader.read_exact(&mut body).unwrap();
                }
                let body = String::from_utf8_lossy(&body).to_string();

                let request = RecordedRequest {
                    method,
                    path,
                    authorization,
                    body,
                };
                let (status, response_body) = handler(&request);
                requests.push(request);

                let status_text = if status == 200 { "OK" } else { "Not Found" };
                let response = format!(
                    "HTTP/1.1 {status} {status_text}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
                    response_body.len()
                );
                writer.write_all(response.as_bytes()).unwrap();
                writer.flush().unwrap();
            }
            requests
        });
        (format!("http://127.0.0.1:{port}"), handle)
    }

    #[gtest]
    #[rstest]
    fn parse_extracts_a_flag_value() {
        let args = parse(&["--name".to_string(), "value".to_string()]).unwrap();
        expect_that!(args.get("name"), eq(Some("value")));
    }

    #[gtest]
    #[rstest]
    fn parse_rejects_arguments_without_a_dash_prefix() {
        expect_that!(
            parse(&["foo".to_string()]),
            err(contains_substring("unexpected"))
        );
    }

    #[gtest]
    #[rstest]
    fn parse_requires_a_value_for_the_last_flag() {
        expect_that!(
            parse(&["--name".to_string()]),
            err(contains_substring("needs a value"))
        );
    }

    #[gtest]
    #[rstest]
    fn write_json_round_trips_through_read_json() {
        let dir = temp_dir();
        let path = dir.path().join("value.json");
        let value = serde_json::json!({ "answer": 42 });

        write_json(&path, &value).unwrap();
        let read: serde_json::Value = read_json(&path).unwrap();

        expect_that!(read, eq(&value));
    }

    #[gtest]
    #[rstest]
    fn read_json_reports_missing_file() {
        let dir = temp_dir();
        let path = dir.path().join("missing.json");

        let result: Result<serde_json::Value, _> = read_json(&path);

        expect_that!(result, err(contains_substring("missing.json")));
    }

    #[gtest]
    #[rstest]
    #[timeout(Duration::from_secs(10))]
    fn keygen_writes_public_key_and_share_files() {
        let keys_dir = temp_dir();
        let shares_dir = temp_dir();

        let result = run(vec![
            "keygen".to_string(),
            "--keys-out".to_string(),
            keys_dir.path().to_str().unwrap().to_string(),
            "--shares-out".to_string(),
            shares_dir.path().to_str().unwrap().to_string(),
            "--period".to_string(),
            TEST_PERIOD.get().to_string(),
            "--threshold".to_string(),
            "2".to_string(),
            "--holders".to_string(),
            "3".to_string(),
        ]);

        expect_that!(result, eq(ExitCode::SUCCESS));
        let key_path = keys_dir
            .path()
            .join(format!("{}.pub", TEST_PERIOD.get()));
        expect_that!(
            PeriodPublicKey::from_base64url(&fs::read_to_string(&key_path).unwrap()),
            ok(anything())
        );
        for holder in 1..=3 {
            let share_path = shares_dir
                .path()
                .join(format!("{}-holder-{holder}.json", TEST_PERIOD.get()));
            expect_that!(
                read_json::<ShareFile>(&share_path),
                ok(field!(ShareFile.holder, eq(&holder)))
            );
        }
    }

    #[gtest]
    #[rstest]
    #[timeout(Duration::from_secs(10))]
    fn keygen_refuses_to_overwrite_an_existing_key(
        keys: billing_ceremony::KeyGeneration,
    ) {
        let keys_dir = temp_dir();
        let shares_dir = temp_dir();
        write_key_file(&keys_dir, TEST_PERIOD, &keys.public_key);

        let result = run(vec![
            "keygen".to_string(),
            "--keys-out".to_string(),
            keys_dir.path().to_str().unwrap().to_string(),
            "--shares-out".to_string(),
            shares_dir.path().to_str().unwrap().to_string(),
            "--period".to_string(),
            TEST_PERIOD.get().to_string(),
        ]);

        expect_that!(result, eq(ExitCode::FAILURE));
    }

    #[gtest]
    #[rstest]
    #[timeout(Duration::from_secs(10))]
    fn run_tally_opens_a_batch_and_writes_totals(keys: billing_ceremony::KeyGeneration) {
        let work = temp_dir();
        let batch = batch_for(
            &keys.public_key,
            &[
                (10_000, "https://a.example"),
                (2_500, "https://b.example:8443"),
            ],
        );
        let batch_path = write_batch_file(&work, &batch);
        let share_paths = write_share_files(&work, &keys.shares[..2]);
        let out_path = work.path().join("totals.json");

        let result = run(vec![
            "tally".to_string(),
            "--batch".to_string(),
            batch_path.to_str().unwrap().to_string(),
            "--share".to_string(),
            share_paths[0].to_str().unwrap().to_string(),
            "--share".to_string(),
            share_paths[1].to_str().unwrap().to_string(),
            "--out".to_string(),
            out_path.to_str().unwrap().to_string(),
        ]);

        expect_that!(result, eq(ExitCode::SUCCESS));
        let totals: Totals = read_json(&out_path).unwrap();
        expect_that!(
            totals.per_site_millicents,
            elements_are![
                (eq("https://a.example"), eq(&10_000)),
                (eq("https://b.example:8443"), eq(&2_500)),
            ]
        );
    }

    #[gtest]
    #[rstest]
    #[timeout(Duration::from_secs(10))]
    fn describe_includes_status_code_and_text() {
        let (url, handle) = fake_admin_server(|_| (404, "not found"), 1);
        let err = ureq::get(&url).call().unwrap_err();

        let message = describe(err);

        expect_that!(
            message,
            contains_substring("admin gateway answered 404")
        );
        handle.join().unwrap();
    }

    #[gtest]
    #[rstest]
    #[timeout(Duration::from_secs(10))]
    fn upload_keys_sends_put_with_public_key(keys: billing_ceremony::KeyGeneration) {
        let work = temp_dir();
        let key_path = write_key_file(&work, TEST_PERIOD, &keys.public_key);
        let (url, handle) = fake_admin_server(|_| (200, ""), 1);

        let result = run(vec![
            "upload-keys".to_string(),
            "--api".to_string(),
            url,
            "--token".to_string(),
            TEST_TOKEN.to_string(),
            "--key".to_string(),
            key_path.to_str().unwrap().to_string(),
        ]);

        expect_that!(result, eq(ExitCode::SUCCESS));
        let requests = handle.join().unwrap();
        expect_that!(requests, elements_are![anything()]);
        let request = &requests[0];
        let public_key = keys.public_key.to_base64url();
        expect_that!(request.method, eq("PUT"));
        expect_that!(
            request.path,
            eq(&format!(
                "/admin/billing/periods/{}/key",
                TEST_PERIOD.get()
            ))
        );
        expect_that!(
            request.authorization,
            eq(&format!("Bearer {TEST_TOKEN}"))
        );
        expect_that!(request.body, contains_substring(public_key.as_str()));
    }

    #[gtest]
    #[rstest]
    #[timeout(Duration::from_secs(10))]
    fn fetch_batch_saves_the_returned_batch() {
        let work = temp_dir();
        let sealed = SealedUsage::try_from(vec![0; 304]).unwrap();
        let batch = Batch {
            period: TEST_PERIOD,
            records: vec![BatchRecord::new(1_000, &sealed)],
        };
        let response_body: &'static str = Box::leak(
            serde_json::to_string(&batch)
                .unwrap()
                .into_boxed_str(),
        );
        let (url, handle) = fake_admin_server(move |_| (200, response_body), 1);
        let out_path = work.path().join("batch.json");

        let result = run(vec![
            "fetch-batch".to_string(),
            "--api".to_string(),
            url,
            "--token".to_string(),
            TEST_TOKEN.to_string(),
            "--period".to_string(),
            TEST_PERIOD.get().to_string(),
            "--out".to_string(),
            out_path.to_str().unwrap().to_string(),
        ]);

        expect_that!(result, eq(ExitCode::SUCCESS));
        let requests = handle.join().unwrap();
        expect_that!(requests, elements_are![anything()]);
        expect_that!(requests[0].method, eq("GET"));
        expect_that!(
            requests[0].path,
            eq(&format!(
                "/admin/billing/periods/{}/batch",
                TEST_PERIOD.get()
            ))
        );
        let saved: Batch = read_json(&out_path).unwrap();
        expect_that!(saved.period, eq(batch.period));
        expect_that!(saved.records, elements_are![anything()]);
    }

    #[gtest]
    #[rstest]
    #[timeout(Duration::from_secs(10))]
    fn upload_totals_sends_the_totals_file() {
        let work = temp_dir();
        let totals = Totals {
            period: TEST_PERIOD,
            per_site_millicents: [("https://a.example".to_string(), 5_000)]
                .into_iter()
                .collect(),
            records: 1,
            dropped: 0,
        };
        let totals_path = work.path().join("totals.json");
        write_json(&totals_path, &totals).unwrap();
        let (url, handle) = fake_admin_server(
            |_| (200, "{\"chargedCents\":100,\"paidOutCents\":90}"),
            1,
        );

        let result = run(vec![
            "upload-totals".to_string(),
            "--api".to_string(),
            url,
            "--token".to_string(),
            TEST_TOKEN.to_string(),
            "--totals".to_string(),
            totals_path.to_str().unwrap().to_string(),
        ]);

        expect_that!(result, eq(ExitCode::SUCCESS));
        let requests = handle.join().unwrap();
        expect_that!(requests, elements_are![anything()]);
        expect_that!(requests[0].method, eq("POST"));
        expect_that!(
            requests[0].path,
            eq(&format!(
                "/admin/billing/periods/{}/totals",
                TEST_PERIOD.get()
            ))
        );
        expect_that!(
            requests[0].body,
            contains_substring("https://a.example")
        );
    }

    #[gtest]
    #[rstest]
    fn run_returns_usage_error_for_unknown_command() {
        let result = run(vec!["unknown".to_string()]);

        expect_that!(result, eq(ExitCode::FAILURE));
    }

    #[gtest]
    #[rstest]
    fn run_returns_usage_error_when_no_command_is_given() {
        let result = run(vec![]);

        expect_that!(result, eq(ExitCode::FAILURE));
    }
}
