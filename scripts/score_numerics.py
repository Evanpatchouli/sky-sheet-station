"""Independent raw-input/Schema/numeric checks, not a full score validator.

N2 rounding uses integer arithmetic, never the Rust decoder or Python float as
the oracle. The host float decoder is a separate, checked observation.
"""

from dataclasses import dataclass
from decimal import Decimal
from fractions import Fraction
import json
import re
import struct

from jsonschema import Draft202012Validator, validators
from referencing import Registry

MAX_TICK = 2**53 - 1
MAX_BITS = 0x7FEFFFFFFFFFFFFF
INTEGER_TOKEN = re.compile(r"(?:0|[1-9][0-9]*)\Z", re.ASCII)


@dataclass(frozen=True)
class Number:
    raw: str


class Pairs(list):
    """An object before duplicate member information has been discarded."""


class InvalidInput(ValueError):
    def __init__(self, stage, path, reason, token=None):
        self.detail = dict(stage=stage, path=path, reason=reason, token=token)
        super().__init__(reason)


def pointer(path, key):
    return path + "/" + str(key).replace("~", "~0").replace("/", "~1")


def parse_raw(raw):
    def constant(value):
        raise InvalidInput("json", "", "non-JSON numeric constant", value)

    def unpack(value, path=""):
        if isinstance(value, Pairs):
            result = {}
            for key, item in value:
                location = pointer(path, key)
                if key in result:
                    raise InvalidInput("raw", location, "duplicate decoded member")
                result[key] = unpack(item, location)
            return result
        if isinstance(value, list):
            return [unpack(item, pointer(path, i)) for i, item in enumerate(value)]
        return value

    return unpack(json.loads(raw, parse_int=Number, parse_float=Number,
                             parse_constant=constant, object_pairs_hook=Pairs))


def schema_view(value):
    if isinstance(value, Number):
        # Decimal construction/comparison is exact, independent of its arithmetic
        # context precision. Never perform baseline arithmetic in Decimal.
        return Decimal(value.raw)
    if isinstance(value, dict):
        return {key: schema_view(item) for key, item in value.items()}
    if isinstance(value, list):
        return [schema_view(item) for item in value]
    return value


def integer_type(checker, value):
    if isinstance(value, bool):
        return False
    if isinstance(value, Decimal):
        return value.is_finite() and value == value.to_integral_value()
    return Draft202012Validator.TYPE_CHECKER.is_type(value, "integer")


def make_validator(schema):
    Draft202012Validator.check_schema(schema)
    cls = validators.extend(
        Draft202012Validator,
        type_checker=Draft202012Validator.TYPE_CHECKER.redefine("integer", integer_type),
    )

    def no_retrieve(uri):
        raise RuntimeError(f"remote Schema retrieval disabled: {uri}")

    return cls(schema, registry=Registry(retrieve=no_retrieve))


def round_even(numerator, denominator):
    quotient, remainder = divmod(numerator, denominator)
    if 2 * remainder > denominator or (2 * remainder == denominator and quotient % 2):
        quotient += 1
    return quotient


def binary64_bits(token):
    """Exact positive-decimal -> nearest binary64, ties even.

    Return None for nonpositive, decoded zero or infinity. This oracle operates
    on verified JSON number tokens, not arbitrary Python float syntax.
    """
    decimal = Decimal(token)
    if not decimal.is_finite() or decimal <= 0:
        return None
    # Avoid constructing astronomical powers for clear overflow/underflow.
    if decimal.adjusted() > 308 or decimal.adjusted() < -324:
        return None
    exact = Fraction(decimal)
    n, d = exact.numerator, exact.denominator
    exponent = n.bit_length() - d.bit_length()
    below_power = n < d << exponent if exponent >= 0 else n << -exponent < d
    if below_power:
        exponent -= 1
    if exponent < -1022:
        significand = round_even(n << 1074, d)
        return significand or None  # Includes rounding up to minimum normal.
    shift = exponent - 52
    significand = round_even(n, d << shift) if shift >= 0 else round_even(n << -shift, d)
    if significand == 2**53:
        significand >>= 1
        exponent += 1
    if exponent > 1023:
        return None
    return ((exponent + 1023) << 52) | (significand - 2**52)


def host_bits(token):
    value = float(token)
    return struct.pack(">d", value).hex()


def bits_fraction(bits):
    exponent, fraction = bits >> 52, bits & (2**52 - 1)
    if exponent == 0:
        return Fraction(fraction, 2**1074)
    significand = 2**52 + fraction
    shift = exponent - 1023 - 52
    return Fraction(significand << shift) if shift >= 0 else Fraction(significand, 2**-shift)


def decimal_text(value):
    """Exact terminating decimal for dyadic boundary fixtures; no float conversion."""
    n, d = value.numerator, value.denominator
    assert d & (d - 1) == 0
    places = d.bit_length() - 1
    if not places:
        return str(n)
    digits = str(n * 5**places).zfill(places + 1)
    return digits[:-places] + "." + digits[-places:]


def analyse(raw, validator):
    """'valid' covers only raw rules, Schema and numeric semantics.

    Unhandled capability/implementation exceptions are tool errors at the runner,
    never reclassified as invalid score data.
    """
    try:
        tree = parse_raw(raw)
    except json.JSONDecodeError as error:
        return dict(status="invalid", schema_valid=None, stage="json", path="",
                    reason=str(error))
    except InvalidInput as error:
        return dict(status="invalid", schema_valid=None, **error.detail)
    errors = list(validator.iter_errors(schema_view(tree)))
    if errors:
        error = errors[0]
        path = ""
        for part in error.absolute_path:
            path = pointer(path, part)
        return dict(status="invalid", schema_valid=False, stage="schema", path=path,
                    reason=error.message)
    try:
        fields = [("/timeline/ppq", tree["timeline"]["ppq"])]
        fields.extend((f"/timeline/time_signature/{name}", tree["timeline"]["time_signature"][name])
                      for name in ("numerator", "denominator"))
        for ti, track in enumerate(tree["tracks"]):
            for ni, note in enumerate(track["notes"]):
                fields.extend((f"/tracks/{ti}/notes/{ni}/{name}", note[name])
                              for name in ("start_tick", "duration_ticks"))
        for path, value in fields:
            if not isinstance(value, Number) or not INTEGER_TOKEN.fullmatch(value.raw):
                raise InvalidInput("raw", path, "plain unsigned integer token required",
                                   value.raw if isinstance(value, Number) else repr(value))
        for ti, track in enumerate(tree["tracks"]):
            for ni, note in enumerate(track["notes"]):
                start, duration = int(note["start_tick"].raw), int(note["duration_ticks"].raw)
                if start + duration > MAX_TICK:
                    raise InvalidInput("numeric", f"/tracks/{ti}/notes/{ni}",
                                       "derived note end exceeds MAX_TICK",
                                       f"{start} + {duration}")
        token = tree["timeline"]["tempo"].raw
        bits = binary64_bits(token)
        if bits is None:
            raise InvalidInput("numeric", "/timeline/tempo", "not finite positive decoded binary64", token)
        expected = f"{bits:016x}"
        if host_bits(token) != expected:
            raise RuntimeError(f"Python decoder differs from exact oracle: {token}; expected {expected}")
        return dict(status="valid", schema_valid=True, tempo_bits=expected, token=token)
    except InvalidInput as error:
        return dict(status="invalid", schema_valid=True, **error.detail)
