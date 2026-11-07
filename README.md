# promptscan

[![CI](https://github.com/lulzis/promptscan/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/lulzis/promptscan/actions/workflows/ci.yml)
[![CodeQL](https://github.com/lulzis/promptscan/actions/workflows/codeql.yml/badge.svg?branch=main)](https://github.com/lulzis/promptscan/actions/workflows/codeql.yml)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/lulzis/promptscan/badge)](https://scorecard.dev/viewer/?uri=github.com/lulzis/promptscan)
[![Rust 1.88+](https://img.shields.io/badge/rust-1.88%2B-orange?logo=rust)](Cargo.toml)
[![Windows 10/11 x64](https://img.shields.io/badge/Windows-10%20%7C%2011%20x64-0078D4)](docs/WEITERGABE.md#2-mindestanforderungen)
[![Network: none](https://img.shields.io/badge/network-none-2ea44f)](tests/offline.rs)
[![Licence: test version](https://img.shields.io/badge/licence-test%20version-lightgrey)](LICENSE.md)

[Deutsch](README.de.md) · **English**

promptscan checks a prompt before it is sent to an AI model. It marks the passages that,
under Swiss law, should not leave your hands without further thought, and cites for every
finding the provision it follows from. It recognises German, French and Italian text.

promptscan runs locally in the terminal. It contains no network code, starts no other
programs and writes files only when explicitly told to.

> **Test version 0.1.** Detection quality has not been measured yet. promptscan reports
> categories of data, not legal violations, and is no substitute for legal advice.

## Example

Fictitious prompt, output without colour (`--lang en`):

```text
> promptscan --lang en draft.txt
Checked locally · Role: profession open, from «Mein Mandant»
Target: cloud

   │ Mein Mandant Peter Muster, geboren am 3. Mai 1971, wohnhaft an der
   │ N^^^^^^^^^^^ R^^^^^^^^^^^             R^^^^^^^^^^
   │ Bahnhofstrasse 12 in 8001 Zürich, AHV-Nr. 756.1234.5678.97, leidet an einer
   │ R^^^^^^^^^^^^^^^^                         R^^^^^^^^^^^^^^^
   │ Depression. Darf ihm der Arbeitgeber kündigen?
   │ R^^^^^^^^^
     R RED · N NOTE

RED       Profession open: Art. 321 Ziff. 1 StGB (e.g. lawyer, psychologist),
          Art. 320 Ziff. 1 StGB (authority), Art. 47 Abs. 1 lit. a BankG (bank)
          Name          Peter Muster
          Date of birth 3. Mai 1971
          Address       Bahnhofstrasse 12
          AHV number    756.1234.5678.97 (check digit valid)
          also sensitive personal data, Art. 5 lit. c Ziff. 2 DSG:
                        Depression

NOTE      Legal consequence not assessed
          Context       «Mein Mandant» (own client)
                        Profession open

Result    RED   Duty of confidentiality backed by criminal law (profession open)
          6 findings: 5 red, 1 note
```

## Levels

| Level | Meaning | Legal basis |
| --- | --- | --- |
| RED | duty of confidentiality backed by criminal law; only with `--role` or inferred from the text | Art. 320, 321 StGB; Art. 47 BankG and others |
| ORANGE | sensitive personal data | Art. 5 lit. c DSG (FADP) |
| YELLOW | personal data | Art. 5 lit. a DSG (FADP) |
| NOTE | legal consequence not assessed | – |

`promptscan rules` lists all rules with their legal basis; `promptscan explain <RULE>` shows
the statutory text, the method, the sources of the word list and the limits of a rule.

## Usage

```text
promptscan                    paste the prompt, Enter, then Ctrl+Z and Enter
promptscan FILE               check a file, UTF-8 or UTF-16 with BOM
promptscan -t "TEXT"          check text directly (it then ends up in the shell history)
promptscan rules              all rules with their legal basis
promptscan explain RULE       provision, method, sources and limits, e.g. ahv or red
promptscan config --role lawyer
                              set the professional role permanently
promptscan licenses           licence status and notices for third-party components

--role ROLE                   professional role with a duty of secrecy, e.g. lawyer, doctor, bank
--target cloud|local          where the prompt goes (default cloud)
--lang de|en                  language of the output (default de)
--format text|json            json for programs, in plain ASCII
--output FILE                 write the output to FILE, UTF-8 with BOM, without colour
--quiet                       only the result, without prompt and findings
--fail-on LEVEL               exit code 1 from red (default), orange or yellow; never: always 0
--no-color, --ascii           no colour; ASCII characters only
```

Do not pass confidential text with `-t`; paste it or check it as a file instead, because
text on the command line ends up in the shell history. Exit code 0 means "nothing at or
above the threshold", 1 "finding at or above the threshold", 2 "error". All options:
`promptscan --help` and [docs/AUSGABE.md](docs/AUSGABE.md).

## Installation

With Rust (see [Building](#building)), Cargo places the executable in
`%USERPROFILE%\.cargo\bin`, which is already on the path:

```text
cargo install --path . --locked
```

For a distributed `promptscan.exe` (compare the checksum first,
[docs/WEITERGABE.md](docs/WEITERGABE.md)), copy it once into a folder of its own and add that
folder to the user path; in PowerShell, in the folder containing the file:

```powershell
$ziel = "$env:LOCALAPPDATA\Programs\promptscan"
New-Item -ItemType Directory -Force $ziel | Out-Null
Copy-Item .\promptscan.exe $ziel
$pfad = [Environment]::GetEnvironmentVariable("Path", "User")
if (";$pfad;" -notlike "*;$ziel;*") {
    [Environment]::SetEnvironmentVariable("Path", "$pfad;$ziel", "User")
}
```

In a new terminal window, `promptscan` then runs without a path.

## Principles

- **Every rule has a legal basis** in the official text currently in force, with the date
  of the version.
- **Every detection has a documented method:** check digit, official format or a word list
  in which every entry carries its source.
- **Everything stays local.** [`tests/offline.rs`](tests/offline.rs) checks the source code,
  `Cargo.lock`, `Cargo.toml` and the import table of the executable to make sure promptscan
  sends nothing and starts no programs.
- **No invented numbers.** What has not been measured is called "not measured".

The specification with the legal model, sources and review status is in
[docs/SPEC.md](docs/SPEC.md) (German).

## Building

Requirements: Windows 10 or 11 (x64), Rust 1.88 or later with the target
`x86_64-pc-windows-gnu`, and `dlltool` from the MinGW-w64 binutils, which `windows-sys` and
`getrandom` use to generate their import libraries. The `dlltool` shipped with the Rust
toolchain needs an assembler that the toolchain does not include; with MSYS2, for example:

```text
pacman -S mingw-w64-ucrt-x86_64-binutils
```

and in `%USERPROFILE%\.cargo\config.toml`:

```toml
[target.x86_64-pc-windows-gnu]
rustflags = ["-C", "dlltool=C:/msys64/ucrt64/bin/dlltool.exe"]
```

Then:

```text
cargo test --release --locked
cargo clippy --release --locked --all-targets -- -D warnings
```

The distributed executable is built by `tools\freigabe.ps1`. The script removes developer
paths, checks the import table and writes the SHA-256 checksum
([docs/WEITERGABE.md](docs/WEITERGABE.md)).

## Documentation

The documentation is in German.

| File | Contents |
| --- | --- |
| [docs/SPEC.md](docs/SPEC.md) | purpose, legal model, rules, review status of the sources |
| [docs/AUSGABE.md](docs/AUSGABE.md) | output, colours, JSON, invocation and configuration |
| [docs/WEITERGABE.md](docs/WEITERGABE.md) | building, checking and distributing the executable |
| [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) | licences of third-party components and data |
| [SECURITY.md](SECURITY.md) | reporting security vulnerabilities |

## Licence

No licence has been chosen for promptscan yet. Until then, any distribution of the executable
or the source code requires the prior consent of the rights holder
([LICENSE.md](LICENSE.md)). Third-party components are subject to their own licences
([THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)).
