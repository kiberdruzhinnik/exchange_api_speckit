use bytes::Bytes;
use exchange_api::cache_store::CacheStore;
use std::time::Duration;

#[tokio::test]
async fn reopened_store_returns_unexpired_complete_response() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite3");
    let response = Bytes::from_static(b"[{\"date\":\"2026-10-06T00:00:00Z\"}]");
    {
        let store = CacheStore::open(&path, 1024).await.unwrap();
        store
            .put("SBER", &response, Duration::from_secs(60))
            .await
            .unwrap();
    }
    let reopened = CacheStore::open(path, 1024).await.unwrap();
    assert_eq!(reopened.get("SBER").await.unwrap().unwrap().body, response);
}

#[tokio::test]
async fn replaces_whole_body_and_prunes_oldest_entry_to_capacity() {
    let dir = tempfile::tempdir().unwrap();
    let store = CacheStore::open(dir.path().join("history.sqlite3"), 6)
        .await
        .unwrap();
    store
        .put("AAA", &Bytes::from_static(b"old!"), Duration::from_secs(60))
        .await
        .unwrap();
    store
        .put("AAA", &Bytes::from_static(b"new!"), Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(
        store.get("AAA").await.unwrap().unwrap().body,
        Bytes::from_static(b"new!")
    );
    store
        .put(
            "BBB",
            &Bytes::from_static(b"other"),
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    assert_eq!(store.get("AAA").await.unwrap(), None);
    assert_eq!(
        store.get("BBB").await.unwrap().unwrap().body,
        Bytes::from_static(b"other")
    );
}

#[tokio::test]
async fn does_not_return_expired_entries() {
    let dir = tempfile::tempdir().unwrap();
    let store = CacheStore::open(dir.path().join("history.sqlite3"), 1024)
        .await
        .unwrap();
    store
        .put("SBER", &Bytes::from_static(b"[]"), Duration::ZERO)
        .await
        .unwrap();
    assert_eq!(store.get("SBER").await.unwrap(), None);
}
