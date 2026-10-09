# signet-forge-dm-qwen35-0.8b-clef-distill-v1

Decision model (DM) for Signet Forge. It picks one id from a closed list: the receiving game's own catalog. It does not invent assets and it does not ship another game's files.

This is the model the lab doors server asks, through Signet Forge, when a Minecraft client has to draw a player who joined from another game, or when a material described in plain words ("green grass lawn") has to become a block or a surface that game already has.

The Signet 2 design on [signetprotocol.io/forge](https://signetprotocol.io/forge/) is still a draft. This model is the piece that already runs in the lab.

## What it is

- Base: Qwen3.5-0.8B.
- Head: the same joint-schema decision head as Cloudflare Clef (`joint_schema_model.py` in this folder).
- Training: LoRA, rank 64, Unsloth, 2 epochs, on 5,493 soft labels. Clef 27B scored the candidates; the student learned Clef's full distribution over those options, not only the winner. 400 rows were held out. Peak training VRAM was about 3.1 GiB. Wall time was about 6 minutes after the labels existed.
- License of these weights: Apache-2.0, same file as `LICENSE` here (the Qwen and Clef head terms that shipped with the export).

The labels and the catalogs they were built from are derived from games. They are not in this repository.

## How a decision is made

1. BM25 keeps the 8 catalog entries closest to the name and the plain-language description.
2. The student scores those 8 and the top score wins. No contextual calibration and no 50/50 mix with BM25. Both of those helped an older, weaker model (Laya) and made this one worse.
3. If BM25 finds fewer than 2 hits, a slower knockout walks the rest of the catalog.
4. A person can approve, reject or preview the choice in Signet Forge. An approval is kept and is not asked again.

At runtime the lab loads it with CUDA graphs (`Student(path, graphs=True)`). On a 4090-class GPU a decision is about 7 ms and about 2 GiB of VRAM. Without the graphs the same model is about 27 ms, because a 0.8B model on a short prompt spends most of its time waiting for Python to launch tiny kernels.

## What we measured

Held-out agreement with Clef, on the 400 rows kept out of training: accuracy 0.68, ECE about 0.029.

On 139 cross-game cases that were never in the training labels (first choice, no calibration):

| | First choice |
|---|---|
| Laya, the previous DM | 51% without calibration, 47% with it |
| Clef 27B, the teacher | 78% |
| This student | 72% |

A sign test on the cases where only one of the two is right: the student 39, Laya 5. The same comparison of Clef against Laya is 40 to 5. The weak pair is Garry's Mod, whose props have almost no description.

A separate set of 47 Lethal Company → Minecraft cases, scored inside Forge after the lab integration: student 79% first choice and 98% in the top 3, with one model call per entity instead of the old knockout.

These figures describe that local eval. They are not a promise about every game.

## Layout

| File | Role |
|---|---|
| `model.safetensors-00001-of-00001.safetensors` | Qwen3.5-0.8B weights, about 1.7 GB. Stored with Git LFS. |
| `joint_head.safetensors` | Decision head |
| `joint_schema_model.py` | Head definition, same contract as Clef |
| `student_train_info.json` | Row counts, loss, held-out calibration |
| `tokenizer.json` | Tokenizer |

## Use

The lab binary does not contain this model. Signet Forge (Python, on the player's PC) loads the folder:

```bash
# from a checkout of this repo
export SIGNET_DM_PATH="$PWD/models/signet-forge-dm-qwen35-0.8b-clef-distill-v1"
```

Forge's own program still lives next to the catalogs on the machine that runs the Link. Point it at this directory instead of a local copy. `git lfs pull` is required before the weights exist on disk.
