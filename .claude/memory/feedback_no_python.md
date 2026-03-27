---
name: No Python in Shadow project
description: Python is not installed and must not be used for any task in this project
type: feedback
---

Do not use Python for anything in the Shadow project. Python is not installed and will not be installed.

**Why:** User explicitly stated Python is not available and won't be added to this environment.

**How to apply:** For scripting tasks (image generation, file manipulation, etc.), use alternatives only: cargo, npm/node, PowerShell, or other tools already available. For icon generation specifically, PowerShell's `System.Drawing` assembly works (already used successfully in M1).
