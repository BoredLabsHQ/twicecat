# Evidence — Task 1: Dependabot configuration
Commit: a4015d7
Environment: local; pipx run check-jsonschema (builtin schema vendor.dependabot); PyYAML to parse; scratch git repo under the session scratchpad (deleted afterwards).

## Four update entries with weekly schedule, limit 5, cooldown 7, prefix fix, include scope; groups and ubuntu ignores
Status: PROVEN
```console
$ python3 (yaml.safe_load of .github/dependabot.yml, print per entry)
cargo / None {'interval': 'weekly', 'day': 'monday'} 5 {'default-days': 7} {'prefix': 'fix', 'include': 'scope'} {'cargo-minor-patch': {'update-types': ['minor', 'patch']}} None
github-actions None ['/', '/.github/actions/setup-build'] {'interval': 'weekly', 'day': 'monday'} 5 {'default-days': 7} {'prefix': 'fix', 'include': 'scope'} {'github-actions': {'patterns': ['*']}} None
docker None ['/dev/ubuntu-24.04', '/dev/ubuntu-24.04-aarch64-cross'] {'interval': 'weekly', 'day': 'monday'} 5 {'default-days': 7} {'prefix': 'fix', 'include': 'scope'} None [{'dependency-name': 'ubuntu', 'update-types': ['version-update:semver-major', 'version-update:semver-minor']}]
docker-compose / None {'interval': 'weekly', 'day': 'monday'} 5 {'default-days': 7} {'prefix': 'fix', 'include': 'scope'} None None
```
Four entries, all fields as specified; header comment also present (read in file).

## Validates against the Dependabot schema
Status: PROVEN
```console
$ pipx run check-jsonschema --builtin-schema vendor.dependabot .github/dependabot.yml
ok -- validation done
exit=0
```

## check-commits.sh exits 0 on the four Dependabot subjects, 1 on build(deps)
Status: PROVEN
```console
$ (scratch repo outside project, branch feat/x off main) git log --format='%h %s' main..HEAD
d34a087 fix(deps): bump matrixdotorg/synapse from v1.151.0 to v1.152.0
4e23fd4 fix(deps-dev): bump tempfile from 3.20.0 to 3.21.0
76c3780 fix(deps): bump actions/checkout from 6.1.0 to 6.2.0
de320f3 fix(deps): bump the cargo-minor-patch group across 1 directory with 3 updates
$ bash scripts/check-commits.sh main; echo "good exit=$?"
good exit=0
$ git commit --allow-empty -m "build(deps): bump x from 1 to 2"; bash scripts/check-commits.sh main; echo "bad exit=$?"
error: not a feat/fix/docs Conventional Commit: 1f0a35a build(deps): bump x from 1 to 2
bad exit=1
```
(An earlier attempt committed on main itself, giving an empty range; discarded and redone on a branch.)

## Only .github/dependabot.yml and PROGRESS.md changed in a4015d7
Status: PROVEN
```console
$ git show --stat --format=%s a4015d7
feat(ci): propose dependency updates with Dependabot

 .github/dependabot.yml | 90 +++++++++++++++++++++++++++++++++++++++++++++++++
 PROGRESS.md            | 21 ++++++++++++
 2 files changed, 111 insertions(+)
```

## GitHub-side checks (Dependabot page, first PRs)
Status: NOT PROVEN (out of scope — HUMAN after merge)

## Verdict
EVIDENCE: PROVEN
