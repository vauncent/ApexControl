"""Query OSV for public crates.io names/versions in Cargo.lock (Python 3.11+).

No source code, local paths, profiles or credentials are transmitted.
This covers known advisories, not an independent code audit.
"""
import argparse
import datetime
import json
from pathlib import Path
import tomllib
import urllib.request


def request(path, payload=None):
    data = None if payload is None else json.dumps(payload).encode()
    req = urllib.request.Request(
        "https://api.osv.dev/v1/" + path,
        data=data,
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=30) as response:
        return json.load(response)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
    packages = [p for p in lock["package"] if p.get("source") == "registry+https://github.com/rust-lang/crates.io-index"]
    findings = []
    for start in range(0, len(packages), 100):
        batch = packages[start : start + 100]
        result = request("querybatch", {"queries": [
            {"package": {"ecosystem": "crates.io", "name": p["name"]}, "version": p["version"]}
            for p in batch
        ]})
        if len(result["results"]) != len(batch):
            raise RuntimeError("Incomplete OSV response; audit did not complete")
        for package, match in zip(batch, result["results"], strict=True):
            # Query individual versions for full results rather than the
            # abbreviated batch entries. Fail closed on truncated detail data.
            if match.get("vulns") or match.get("next_page_token"):
                details = request("query", {"package": {"ecosystem": "crates.io", "name": package["name"]}, "version": package["version"]})
                if details.get("next_page_token"):
                    raise RuntimeError("OSV details require pagination; audit did not complete")
                findings.append({"name": package["name"], "version": package["version"], "advisories": details.get("vulns", [])})
    report = {"checked_at": datetime.datetime.now(datetime.timezone.utc).isoformat(), "source": "https://osv.dev/", "packages_checked": len(packages), "findings": findings}
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"OSV checked {len(packages)} locked crates.io packages; {len(findings)} packages have advisory matches.")
    for finding in findings:
        for advisory in finding["advisories"]:
            print(f"{finding['name']} {finding['version']}: {advisory['id']} {advisory.get('summary', '')}")
    return bool(findings)


if __name__ == "__main__":
    raise SystemExit(main())
