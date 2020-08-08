package main

import (
	"bytes"
	"errors"
	"fmt"
	"os"
	"os/exec"

	git "github.com/go-git/go-git/v5"
	"github.com/go-git/go-git/v5/config"
	"github.com/go-git/go-git/v5/plumbing"
	"github.com/go-git/go-git/v5/plumbing/object"
	"github.com/go-git/go-git/v5/plumbing/transport/http"
	"github.com/sirupsen/logrus"
)

var log = logrus.New()

func run() error {
	github := Remote{
		Name:     "github",
		URL:      "https://github.com/eriktate/kagami-test.git",
		Branch:   "master",
		Username: "eriktate",
		Password: os.Getenv("GITHUB_ACCESS_TOKEN"),
	}

	gitlab := Remote{
		Name:     "gitlab",
		Branch:   "master",
		URL:      "https://gitlab.com/eriktate/kagami-test.git",
		Username: "eriktate",
		Password: os.Getenv("GITLAB_ACCESS_TOKEN"),
	}

	fork, err := CreateFork(github, gitlab)
	if err != nil {
		return err
	}

	patch, err := fork.DiffRemotes()
	if err != nil {
		return err
	}

	log.Info(patch)

	// dry run
	// if err := fork.Merge(true); err != nil {
	// 	return err
	// }

	// if err := fork.Merge(false); err != nil {
	// 	return err
	// }

	// if err := fork.Push(); err != nil {
	// 	return err
	// }

	return nil
}

type Remote struct {
	Name     string
	Branch   string
	URL      string
	Username string
	Password string

	remote *git.Remote
}

func (r Remote) String() string {
	return fmt.Sprintf("%s/%s", r.Name, r.Branch)
}

func (r Remote) ReferenceName() plumbing.ReferenceName {
	return plumbing.ReferenceName(fmt.Sprintf("refs/remotes/%s/%s", r.Name, r.Branch))
}

type Fork struct {
	RemoteA Remote
	RemoteB Remote
	repo    *git.Repository
}

func fetchRemote(remote Remote, repo *git.Repository) error {
	remConf := config.RemoteConfig{
		Name: remote.Name,
		URLs: []string{remote.URL},
	}

	if _, err := repo.CreateRemote(&remConf); err != nil {
		return err
	}

	fetchOutput := bytes.NewBuffer(make([]byte, 1024))
	fetchOpts := git.FetchOptions{
		RemoteName: remote.Name,
		Auth: &http.BasicAuth{
			Username: remote.Username,
			Password: remote.Password,
		},
		Progress: fetchOutput,
	}

	if err := repo.Fetch(&fetchOpts); err != nil {
		return err
	}

	return nil

}

func CreateFork(remoteA Remote, remoteB Remote) (*Fork, error) {
	repo, err := git.PlainInit("./sandbox/fork", false)
	if err != nil {
		if err != git.ErrRepositoryAlreadyExists {
			return nil, err
		}

		repo, err = git.PlainOpen("./sandbox/fork")
		if err != nil {
			return nil, err
		}
	}

	if err := fetchRemote(remoteA, repo); err != nil {
		if err != git.ErrRemoteExists {
			return nil, fmt.Errorf("failed to fetch remote \"%s\": %w", remoteA.Name, err)
		}
	}

	if err := fetchRemote(remoteB, repo); err != nil {
		if err != git.ErrRemoteExists {
			return nil, fmt.Errorf("failed to fetch remote \"%s\": %w", remoteB.Name, err)
		}
	}

	branchConf := config.Branch{
		Name:   remoteB.Branch,
		Remote: remoteB.Name,
		Merge:  plumbing.ReferenceName("refs/heads/" + remoteB.Branch),
	}

	if err := repo.CreateBranch(&branchConf); err != nil {
		return nil, fmt.Errorf("failed to create tracking branch: %w", err)
	}

	return &Fork{
		RemoteA: remoteA,
		RemoteB: remoteB,
		repo:    repo,
	}, nil
}

func getRemoteHead(remote Remote, repo *git.Repository) (*object.Commit, error) {
	ref, err := repo.Reference(remote.ReferenceName(), true)
	if err != nil {
		return nil, fmt.Errorf("failed to resolve branch \"%s\"", remote)
	}

	commit, err := repo.CommitObject(ref.Hash())
	if err != nil {
		return nil, fmt.Errorf("failed to resolve commit for \"%s\"", remote)
	}

	return commit, nil
}

func (f *Fork) DiffRemotes() (*object.Patch, error) {
	commitA, err := getRemoteHead(f.RemoteA, f.repo)
	if err != nil {
		return nil, err
	}

	commitB, err := getRemoteHead(f.RemoteB, f.repo)
	if err != nil {
		return nil, err
	}

	return commitB.Patch(commitA)
}

func (f *Fork) Merge(dryRun bool) (string, error) {
	out := bytes.NewBuffer(make([]byte, 1024))
	var mergeCmd *exec.Cmd
	if dryRun {
		mergeCmd = exec.Command("git", "merge", "--no-commit", "--no-ff", f.RemoteA.String())
	} else {
		mergeCmd = exec.Command("git", "merge", f.RemoteA.String())
	}
	mergeCmd.Dir = "./sandbox/fork"

	if err := mergeCmd.Run(); err != nil {
		return "", errors.New(out.String())
	}

	return out.String(), nil
}

func (f *Fork) Push() error {
	pushOutput := bytes.NewBuffer(make([]byte, 1024))
	pushOpts := git.PushOptions{
		RemoteName: f.RemoteB.Name,
		Auth: &http.BasicAuth{
			Username: f.RemoteB.Username,
			Password: f.RemoteB.Password,
		},
		Progress: pushOutput,
	}

	if err := f.repo.Push(&pushOpts); err != nil {
		return err
	}

	return nil
}

func main() {
	if err := run(); err != nil {
		log.Error(err)
	}
}
