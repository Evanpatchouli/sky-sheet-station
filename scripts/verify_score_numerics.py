"""N1-N4 gate: original JSON -> independent checks and unchanged Rust reader.

Exit 0: requested checks passed; 1: expectation/conformance differences;
2: tool/environment failure. Reports live under ignored target/ by default.
"""

import argparse
import copy
from fractions import Fraction
import importlib.metadata
import json
from pathlib import Path
import random
import re
import subprocess
import sys

try:
    from score_numerics import MAX_BITS, analyse, bits_fraction, decimal_text, make_validator
except ImportError as error:
    if __name__ == "__main__":
        print(f"tool_error: {error}; install requirements/validation.lock", file=sys.stderr)
        sys.exit(2)
    raise

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "crates/skysheet-score/tests/fixtures"
SEED = 0x534B59


def replace_at(value, pointer, replacement):
    if not pointer:
        return replacement
    parts = pointer[1:].split("/")
    parent = value
    for part in parts[:-1]:
        parent = parent[int(part)] if isinstance(parent, list) else parent[part]
    key = int(parts[-1]) if isinstance(parent, list) else parts[-1]
    parent[key] = replacement
    return value


def cases():
    example_text = (ROOT / "docs/examples/sky-15-example.json").read_text(encoding="utf-8")
    example = json.loads(example_text)
    result = []
    for filename in ("contract-boundaries.json", "contract-conformance.json", "object-shapes.json"):
        for case in json.loads((FIXTURES / filename).read_text(encoding="utf-8")):
            value = copy.deepcopy(example)
            for pointer in case.get("removes", []):
                parts = pointer[1:].split("/")
                parent = value
                for part in parts[:-1]:
                    parent = parent[int(part)] if isinstance(parent, list) else parent[part]
                del parent[parts[-1]]
            for pointer, replacement in case["changes"].items():
                value = replace_at(value, pointer, replacement)
            result.append(dict(id="shared/" + case["name"], raw=json.dumps(value, ensure_ascii=False),
                               schema_valid=case["schema_valid"], score_valid=case["rust_error"] is None))
    for case in json.loads((FIXTURES / "numeric-inputs.json").read_text(encoding="utf-8")):
        result.append(dict(case, score_valid=case["valid"]))
    # Exact midpoints plus one quarter-gap on either side, generated without a
    # decoder. Parity determines tie expectations, including zero and infinity.
    for left in [0, 1, 2, 2**52 - 1, 2**52, 0x3FEFFFFFFFFFFFFF,
                 0x3FF0000000000000, 0x3FF0000000000001,
                 0x3FB999999999999A, MAX_BITS - 1, MAX_BITS]:
        a = bits_fraction(left)
        b = Fraction(2**1024) if left == MAX_BITS else bits_fraction(left + 1)
        midpoint = (a + b) / 2
        for label, value, expected in [
            ("below", midpoint - (b - a) / 4, left),
            ("tie", midpoint, left if left % 2 == 0 else left + 1),
            ("above", midpoint + (b - a) / 4, left + 1),
        ]:
            token = decimal_text(value)
            valid = 0 < expected <= MAX_BITS
            result.append(dict(id=f"midpoint/{left:016x}/{label}",
                               raw=example_text.replace('"tempo": 120', '"tempo": ' + token, 1),
                               valid=valid, schema_valid=True, score_valid=valid,
                               bits=f"{expected:016x}" if valid else None))
    assert len({case["id"] for case in result}) == len(result)
    return result


def sampled_bits(count, seed):
    if count < 0:
        raise ValueError("sample count must be nonnegative")
    rng = random.Random(seed)
    values = [1, 2, 2**52 - 1, 2**52, MAX_BITS]
    values.extend((exponent << 52) | rng.getrandbits(52) for exponent in range(2047))
    values = list(dict.fromkeys(value for value in values if value))[:count]
    seen = set(values)
    while len(values) < count:
        value = rng.randint(1, MAX_BITS)
        if value not in seen:
            values.append(value)
            seen.add(value)
    return values


def compare(case, independent, observation, validator):
    """Return concrete disagreements without changing the approved expectations."""
    errors = []
    expected_valid = case["score_valid"]
    for path in ("from_str", "from_reader"):
        actual = observation[path]
        if (actual["status"] == "valid") != expected_valid:
            errors.append(dict(check=path + "/acceptance", expected=expected_valid, actual=actual))
            continue
        if not expected_valid:
            continue
        expected_bits = case.get("bits") or independent.get("tempo_bits")
        if actual["bits"] != expected_bits:
            errors.append(dict(check=path + "/decode_bits", expected=expected_bits, actual=actual["bits"]))
        saved = analyse(actual["serialized"], validator)
        if saved["status"] != "valid" or saved.get("tempo_bits") != actual["bits"]:
            errors.append(dict(check=path + "/serialization", expected=actual["bits"], actual=saved))
        roundtrip = actual["roundtrip"]
        if roundtrip["status"] != "valid" or roundtrip.get("bits") != actual["bits"]:
            errors.append(dict(check=path + "/roundtrip", expected=actual["bits"], actual=roundtrip))
    return errors


def run(args):
    if sys.version_info[:2] != (3, 13) or sys.implementation.name != "cpython":
        raise RuntimeError("locked validation environment requires CPython 3.13")
    if (sys.float_info.radix, sys.float_info.mant_dig, sys.float_info.max_exp) != (2, 53, 1024):
        raise RuntimeError("host float is not binary64")
    locked = re.findall(r"^([\w-]+)==([\w.]+)",
                        (ROOT / "requirements/validation.lock").read_text(encoding="utf-8"), re.M)
    for name, version in locked:
        if importlib.metadata.version(name) != version:
            raise RuntimeError(f"validation dependency differs from lock: {name} must be {version}")
    schema = json.loads((ROOT / "docs/schemas/sky-score-v1.schema.json").read_text(encoding="utf-8"))
    validator = make_validator(schema)
    fixed = cases()
    sample_case = args.case and args.case.startswith("sample/")
    if sample_case:
        if args.independent_only:
            raise ValueError("a sample case requires the Rust serializer")
        fixed = []
        bits = int(args.case.removeprefix("sample/"), 16)
        if not 0 < bits <= MAX_BITS:
            raise ValueError("sample case must encode finite positive binary64")
    elif args.case:
        fixed = [case for case in fixed if case["id"] == args.case]
        if not fixed:
            raise ValueError(f"unknown case: {args.case}")
    failures = []
    expected = {}
    requests = []
    for case in fixed:
        result = analyse(case["raw"], validator)
        checks = []
        if result["schema_valid"] != case["schema_valid"]:
            checks.append(dict(check="independent/schema", expected=case["schema_valid"], actual=result))
        if "valid" in case and (result["status"] == "valid") != case["valid"]:
            checks.append(dict(check="independent/numeric", expected=case["valid"], actual=result))
        if case.get("bits") and result.get("tempo_bits") != case["bits"]:
            checks.append(dict(check="independent/bits", expected=case["bits"], actual=result))
        if checks:
            failures.append(dict(id=case["id"], raw=case["raw"], differences=checks))
        expected[case["id"]] = (case, result)
        requests.append(dict(id=case["id"], raw=case["raw"]))
    sample_values = ([bits] if sample_case else
                     [] if args.case or args.independent_only else sampled_bits(args.samples, args.seed))
    for bits in sample_values:
        identifier = f"sample/{bits:016x}"
        requests.append(dict(id=identifier, bits=f"{bits:016x}"))
        expected[identifier] = (dict(id=identifier, score_valid=True, bits=f"{bits:016x}"), None)
    if not args.independent_only:
        process = subprocess.run(
            ["cargo", "run", "--quiet", "--locked", "-p", "skysheet-score", "--example", "numeric_probe"],
            cwd=ROOT, input="".join(json.dumps(item, ensure_ascii=True) + "\n" for item in requests),
            text=True, encoding="utf-8", capture_output=True, timeout=180,
        )
        if process.stderr:
            print(process.stderr.rstrip(), file=sys.stderr)
        if process.returncode:
            raise RuntimeError(f"Rust observation bridge failed ({process.returncode})")
        seen = set()
        for line in process.stdout.splitlines():
            observation = json.loads(line)
            identifier = observation["id"]
            if identifier in seen or identifier not in expected:
                raise RuntimeError(f"unexpected/duplicate probe response: {identifier}")
            seen.add(identifier)
            case, independent = expected[identifier]
            if independent is None:
                independent = analyse(observation["raw"], validator)
                if independent["status"] != "valid" or independent.get("tempo_bits") != case["bits"]:
                    failures.append(dict(id=identifier, raw=observation["raw"], differences=[
                        dict(check="in_memory/serialization", expected=case["bits"], actual=independent)]))
            differences = compare(case, independent, observation, validator)
            if differences:
                failures.append(dict(id=identifier, raw=observation["raw"],
                                     token=independent.get("token"), differences=differences))
        if seen != set(expected):
            raise RuntimeError("incomplete Rust probe output")
    report = dict(status="differences" if failures else "passed", rust_checked=not args.independent_only,
                  scope="original input, Schema and numeric rules; not full independent score semantics",
                  fixed_cases=len(fixed), samples=len(sample_values), seed=args.seed,
                  python=sys.version, packages={name: importlib.metadata.version(name) for name in
                    ("jsonschema", "attrs", "jsonschema-specifications", "referencing", "rpds-py")},
                  failures=failures)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"{report['status']}: {len(fixed)} fixed cases, {len(sample_values)} samples; "
          f"{len(failures)} failing case records; Rust checked: {report['rust_checked']}")
    for failure in failures[:5]:
        print(json.dumps(dict(id=failure["id"], differences=failure["differences"]), ensure_ascii=True))
    print(f"Report: {args.report}")
    return 1 if failures else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--samples", type=int, default=10000)
    parser.add_argument("--seed", type=lambda value: int(value, 0), default=SEED)
    parser.add_argument("--case", help="run one exact case ID, including sample/<hex bits>")
    parser.add_argument("--independent-only", action="store_true", help="not the complete conformance gate")
    parser.add_argument("--report", type=Path, default=ROOT / "target/numeric-conformance-report.json")
    args = parser.parse_args()
    try:
        return run(args)
    except Exception as error:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(dict(status="tool_error", error=str(error)), indent=2) + "\n",
                               encoding="utf-8")
        print(f"tool_error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
