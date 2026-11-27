// SPDX-License-Identifier: AGPL-3.0-or-later

//! The upload flow and `/inventory` through the conversation layer, with the real parsers on real
//! fixtures (docs/architecture/inventory.md).

#![allow(clippy::unwrap_used)]

mod common;

use common::Instance;

fn fixture(parser: &str, name: &str) -> Vec<u8> {
    let path = format!(
        "{}/../../plugins/parsers/{parser}/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(path).unwrap()
}

#[tokio::test]
async fn uploads_merge_as_a_union_and_refresh_known_entries() {
    // inventory.md#entries-and-maintenance: new pairs are added, known pairs refreshed, nothing
    // removed.
    let instance = Instance::with_admin().await;
    let first = instance
        .upload(
            "alice",
            "npm-shrinkwrap.json",
            fixture("npm", "npm-shrinkwrap-v3.json"),
        )
        .await
        .unwrap();
    assert!(
        first.starts_with("Inventory updated from npm-shrinkwrap.json: 8 new, 0 already known."),
        "{first}"
    );
    // The bun.lock shares typescript 5.9.3 with the first file and adds 22 other pairs.
    let second = instance
        .upload("alice", "bun.lock", fixture("bun", "bun.lock"))
        .await
        .unwrap();
    assert!(
        second.starts_with("Inventory updated from bun.lock: 22 new, 1 already known."),
        "{second}"
    );
    let listed = instance.send("alice", "/inventory").await.unwrap();
    assert!(listed.starts_with("30 entries:"), "{listed}");
    assert!(
        listed.contains("• pkg:npm/clipanion 4.0.0-rc.4"),
        "{listed}"
    );
    assert!(listed.contains("• pkg:npm/chalk 5.6.2"), "{listed}");
}

#[tokio::test]
async fn uploads_add_derived_watches() {
    let instance = Instance::with_admin().await;
    let reply = instance
        .upload(
            "alice",
            "package-lock.json",
            fixture("npm", "package-lock-v3.json"),
        )
        .await
        .unwrap();
    assert!(
        reply.ends_with("Technologies detected, now watched: JavaScript, npm."),
        "{reply}"
    );
    // typescript in a lock file maps to tech:typescript through the taxonomy.
    let reply = instance
        .upload(
            "alice",
            "npm-shrinkwrap.json",
            fixture("npm", "npm-shrinkwrap-v3.json"),
        )
        .await
        .unwrap();
    assert!(
        reply.ends_with("now watched: JavaScript, npm, TypeScript."),
        "{reply}"
    );
    assert_eq!(
        instance.send("alice", "/watching").await.unwrap(),
        "You watch:\n\
         • JavaScript (tech:javascript) · from your uploads\n\
         • npm (tech:npm) · from your uploads\n\
         • TypeScript (tech:typescript) · from your uploads"
    );
}

#[tokio::test]
async fn uploaded_files_are_stored_nowhere() {
    // inventory.md#privacy: files are never stored, in whole or in part; no project name.
    let instance = Instance::with_admin().await;
    instance
        .upload(
            "alice",
            "package-lock.json",
            fixture("npm", "package-lock-v3.json"),
        )
        .await
        .unwrap();
    instance.database.close();
    let mut stored = Vec::new();
    for entry in std::fs::read_dir(instance.dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            stored.extend(std::fs::read(&path).unwrap());
        }
    }
    let stored = String::from_utf8_lossy(&stored);
    // An integrity hash and the project name from the uploaded file.
    assert!(
        stored.contains("pkg:npm/before-after-hook"),
        "entries are stored"
    );
    assert!(
        !stored.contains("sha512-"),
        "file content leaked into the data directory"
    );
    assert!(
        !stored.contains("0.0.0-development"),
        "the project version leaked"
    );
    let inventory = instance.send("alice", "/inventory").await.unwrap();
    assert!(
        !inventory.contains("octokit/core"),
        "the project itself became an entry"
    );
}

#[tokio::test]
async fn rejected_uploads_get_an_explanation() {
    let instance = Instance::with_admin().await;
    let cases: [(&str, Vec<u8>, &str); 4] = [
        (
            "yarn.lock",
            b"# yarn lockfile v1".to_vec(),
            "I cannot read yarn.lock. Supported files:",
        ),
        (
            "bun.lockb",
            vec![0, 1, 2],
            "Bun’s binary lockfile (bun.lockb) is not supported.",
        ),
        (
            "package-lock.json",
            b"{ not json".to_vec(),
            "package-lock.json could not be read",
        ),
        // Synthetic: one byte over the 20 MB limit, checked before parsing.
        (
            "pnpm-lock.yaml",
            vec![b' '; 20 * 1024 * 1024 + 1],
            "The file is larger than 20 MB.",
        ),
    ];
    for (name, bytes, expected) in cases {
        let reply = instance.upload("alice", name, bytes).await.unwrap();
        assert!(reply.starts_with(expected), "{name}: {reply}");
    }
    assert_eq!(
        instance.send("alice", "/inventory").await.unwrap(),
        "Your inventory is empty. Send a lock file or use /inventory add <purl> <version>."
    );
}

#[tokio::test]
async fn manual_entries_and_removal() {
    let instance = Instance::with_admin().await;
    assert_eq!(
        instance
            .send("alice", "/inventory add pkg:npm/lodash 4.17.20")
            .await
            .unwrap(),
        "Added pkg:npm/lodash 4.17.20."
    );
    instance
        .send("alice", "/inventory add pkg:npm/lodash 4.17.21")
        .await
        .unwrap();
    instance
        .send("alice", "/inventory add pkg:npm/%40babel/core")
        .await
        .unwrap();
    assert!(
        instance
            .send("alice", "/inventory add lodash 1.0.0")
            .await
            .unwrap()
            .contains("is not a package URL")
    );
    assert_eq!(
        instance
            .send("alice", "/inventory remove pkg:npm/lodash@4.17.20")
            .await
            .unwrap(),
        "Removed one entry."
    );
    let listed = instance.send("alice", "/inventory").await.unwrap();
    assert!(
        listed.contains("• pkg:npm/%40babel/core (no version)"),
        "{listed}"
    );
    assert!(listed.contains("• pkg:npm/lodash 4.17.21"), "{listed}");
    assert!(!listed.contains("4.17.20"), "{listed}");
    assert_eq!(
        instance.send("alice", "/inventory clear").await.unwrap(),
        "Your inventory was cleared (2 entries removed)."
    );
}

#[tokio::test]
async fn each_subscriber_has_their_own_inventory() {
    let instance = Instance::with_admin().await;
    let invite = instance.invite("alice").await;
    instance
        .send("bob", &format!("/start {invite}"))
        .await
        .unwrap();
    instance
        .send("alice", "/inventory add pkg:npm/lodash 4.17.20")
        .await
        .unwrap();
    assert!(
        instance
            .send("bob", "/inventory")
            .await
            .unwrap()
            .starts_with("Your inventory is empty")
    );
}
