use git2::{Commit, Cred, Diff, Error, FetchOptions, RemoteCallbacks, Repository, Signature};
use std::path::PathBuf;

pub struct Remote {
    name: String,
    url: String,
    branch: String,
    username: String,
    password: String,
}

pub struct Fork {
    repo: Repository,
    path: PathBuf,
    src: Remote,
    dst: Remote,
}

impl Fork {
    pub fn new(src: Remote, dst: Remote, path: Option<&str>) -> Result<Fork, Error> {
        let pathbuf = match path {
            Some(p) => PathBuf::from(p),
            None => PathBuf::from("./sandbox"),
        };

        let repo = init_repo(&pathbuf)?;

        src.init(&repo)?;
        dst.init(&repo)?;

        // setup local tracking branch
        {
            let commit = find_branch_head(&repo, &dst.name, &dst.branch)?;
            let tracking_branch = repo.branch(&dst.name, &commit, false)?;
            let tracking_ref = tracking_branch.into_reference();
            repo.set_head(tracking_ref.name().unwrap())?;
            repo.checkout_head(None)?;
        }

        Ok(Fork {
            repo,
            path: pathbuf,
            src,
            dst,
        })
    }

    pub fn get_diff(&self) -> Result<Diff, Error> {
        let src_commit = find_branch_head(&self.repo, &self.src.name, &self.src.branch)?;
        let dst_commit = find_branch_head(&self.repo, &self.dst.name, &self.dst.branch)?;
        let src_tree = self.repo.find_tree(src_commit.tree_id())?;
        let dst_tree = self.repo.find_tree(dst_commit.tree_id())?;

        self.repo
            .diff_tree_to_tree(Some(&src_tree), Some(&dst_tree), None)
    }

    /// returns whether or not a merge is possible
    pub fn merge(&self) -> Result<bool, Error> {
        let src_commit = find_branch_head(&self.repo, &self.src.name, &self.src.branch)?;
        let src_ann_commit = self.repo.find_annotated_commit(src_commit.id())?;
        let dst_commit = find_branch_head(&self.repo, &self.dst.name, &self.dst.branch)?;
        self.repo.merge(&[&src_ann_commit], None, None)?;
        let mut index = self.repo.index()?;

        let can_merge = !index.has_conflicts();

        let tree_oid = index.write_tree()?;
        let tree = self.repo.find_tree(tree_oid)?;
        let sig = Signature::now("Erik Tate", "hello@eriktate.me")?;
        self.repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Merge commit",
            &tree,
            &[&dst_commit],
        )?;
        self.repo.cleanup_state()?;

        Ok(can_merge)
    }
}

fn init_repo(path: &PathBuf) -> Result<Repository, Error> {
    match Repository::init(&path) {
        Ok(repo) => Ok(repo),
        Err(_) => Repository::open(path),
    }
}

impl Remote {
    pub fn new(name: &str, url: &str, branch: &str, username: &str, password: &str) -> Remote {
        Remote {
            name: String::from(name),
            url: String::from(url),
            branch: String::from(branch),
            username: String::from(username),
            password: String::from(password),
        }
    }
    fn init(&self, repo: &Repository) -> Result<(), Error> {
        let mut remote = match repo.remote(&self.name, &self.url) {
            Ok(remote) => remote,
            Err(_) => repo.find_remote(&self.name)?,
        };

        let mut auth_cb = RemoteCallbacks::new();
        auth_cb.credentials(|_url, _username_from_url, _allowed_types| {
            Cred::userpass_plaintext(&self.username, &self.password)
        });

        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(auth_cb);

        remote.fetch(&[&self.branch], Some(&mut fetch_opts), None)
    }
}

fn find_branch_head<'repo>(
    repo: &'repo Repository,
    remote: &str,
    branch: &str,
) -> Result<Commit<'repo>, Error> {
    let obj = repo.revparse_single(&format!("{}/{}", remote, branch))?;

    let commit = obj.into_commit().unwrap();
    Ok(commit)
}
