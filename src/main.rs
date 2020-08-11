use git2::{
    BranchType, Commit, Cred, DiffFormat, Error, FetchOptions, Remote, RemoteCallbacks, Repository,
    Signature,
};
use std::path::{Path, PathBuf};
// use std::env;

mod fork;

fn main() {
    let github_password = env!("GITHUB_ACCESS_TOKEN", "no github token provided");
    let gitlab_password = env!("GITLAB_ACCESS_TOKEN", "no gitlab token provided");

    let gh_remote = fork::Remote::new(
        "github",
        "https://github.com/eriktate/kagami-test.git",
        "master",
        "eriktate",
        github_password,
    );

    let gl_remote = fork::Remote::new(
        "gitlab",
        "https://gitlab.com/eriktate/kagami-test.git",
        "master",
        "eriktate",
        gitlab_password,
    );

    let fork = fork::Fork::new(gh_remote, gl_remote, None).unwrap();

    let diff = fork.get_diff().unwrap();
    let mut diff_text = String::new();
    diff.print(DiffFormat::Patch, |delta, hunk, line| {
        println!("Delta: {:?}", delta);
        println!("Hunk: {:?}", hunk);
        println!("Line: {:?}", line);
        diff_text.push_str(std::str::from_utf8(line.content()).unwrap());
        true
    });

    println!("Diff Text: {}", diff_text);

    fork.merge();
    // merge!
    // repo.merge(&[&gh_ann_commit], None, None).unwrap();
    // let mut index = repo.index().unwrap();
    // if index.has_conflicts() {
    //     // capture conflicts
    // }

    // let tree_oid = index.write_tree().unwrap();
    // let tree = repo.find_tree(tree_oid).unwrap();
    // let sig = Signature::now("Erik Tate", "hello@eriktate.me").unwrap();
    // repo.commit(Some("HEAD"), &sig, &sig, "Merge commit", &tree, &[&commit])
    //     .unwrap();
    // repo.cleanup_state();
}
