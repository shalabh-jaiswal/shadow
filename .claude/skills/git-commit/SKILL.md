---
name: git-commit
description: Automatically checks for changes, stages them, analyzes the diff to generate a Conventional Commit message, and commits the code.
author: Gemini
version: 1.0.0
---

# Skill: git-commit

This skill automates the Git commit workflow. It prioritizes CLI and token efficiency by verifying changes exist before staging, and by excluding massive auto-generated files from the AI's context window.

## Workflow

### 1. Fast-Fail Check
* **Command:** Run `git status --porcelain`.
* **Action:** If the output is empty, STOP execution immediately. Notify the user: "Working tree is clean. Nothing to commit."

### 2. Automatic Staging
* **Command:** Run `git add .` to stage all modified, deleted, and untracked files.

### 3. Context Extraction (The Diff)
* **Command:** Run `git diff --cached` to extract the exact code changes.
* **Token Optimization:** When running the diff or analyzing it, explicitly ignore large auto-generated files (e.g., `package-lock.json`, `yarn.lock`, `poetry.lock`, compiled binaries, or minified assets) as they consume excessive tokens and do not help generate a better commit message.

### 4. Message Generation
Analyze the diff and generate a commit message strictly following **Conventional Commits**:
* **Format:** `<type>(<scope>): <description>`
* **Types:** * `feat`: A new feature
  * `fix`: A bug fix
  * `docs`: Documentation only changes
  * `style`: Formatting/white-space (no code logic changes)
  * `refactor`: Code change that neither fixes a bug nor adds a feature
  * `perf`: Performance improvement
  * `test`: Adding or correcting tests
  * `chore`: Build process or auxiliary tool changes
* **Rules:**
  * Use the imperative, present tense in the description: "add" not "added" or "adds".
  * Do not capitalize the first letter of the description.
  * Do not put a period (.) at the end of the description.
  * If the diff is complex, include a brief body formatted with a blank line after the description to explain the *why* behind the change.
  * Never add Co-Authored-By lines to git commits. 
  * Scopes (optional): `daemon`, `watcher`, `providers`, `s3`, `gcs`, `nas`, `ipc`, `frontend`, `ci`



### 5. Execution
* **Command:** Execute `git commit -m "<generated_message>"`. If a body is included, use `git commit -m "<description>" -m "<body>"`.
* **Output:** Return the short hash and the commit message to the user to confirm success.

---

## Agent Instructions

1. Run `git status --porcelain`.
2. If the output is empty, halt and report: "No changes detected. Working tree clean."
3. If changes exist, run `git add .`.
4. Run `git diff --cached` (excluding lock files/binaries) and read the output.
5. Generate a Conventional Commit message based on the exact code changes in the diff.
6. Execute the commit using `git commit -m "[generated_message]"`.