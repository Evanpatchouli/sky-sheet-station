"""Self-tests of the independent checker, including deliberate wrong observations."""

import copy
from decimal import localcontext
from fractions import Fraction
import json
import unittest

from score_numerics import (
    MAX_BITS, InvalidInput, Number, analyse, binary64_bits, bits_fraction,
    decimal_text, make_validator, parse_raw, round_even, schema_view,
)
from verify_score_numerics import ROOT, cases, compare, sampled_bits, SEED


class NumericChecks(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = json.loads((ROOT / "docs/schemas/sky-score-v1.schema.json").read_text(encoding="utf-8"))
        cls.validator = make_validator(cls.schema)
        cls.example = (ROOT / "docs/examples/sky-15-example.json").read_text(encoding="utf-8")

    def test_raw_number_tokens_are_preserved(self):
        values = parse_raw('[1,1.0,1e0,-0,true,"1"]')
        self.assertEqual([v.raw for v in values[:4]], ["1", "1.0", "1e0", "-0"])
        self.assertIs(values[4], True)
        self.assertEqual(values[5], "1")

    def test_decoded_duplicate_names_and_scope(self):
        for raw in [r'{"name":null,"\u006eame":null}', '{"a":{"x":1,"x":2}}']:
            with self.subTest(raw=raw), self.assertRaises(InvalidInput):
                parse_raw(raw)
        result = parse_raw('{"Name":1,"name":2,"é":3,"e\\u0301":4,"other":{"name":5}}')
        self.assertEqual(len(result), 5)  # No case folding or Unicode normalization.

    def test_json_grammar_is_not_python_float_grammar(self):
        for token in ["NaN", "Infinity", "-Infinity", "+1", "01", "1.", "1e"]:
            with self.subTest(token=token):
                result = analyse(self.example.replace('"tempo": 120', '"tempo": ' + token), self.validator)
                self.assertEqual(result["stage"], "json")
                self.assertIsNone(result["schema_valid"])

    def test_decimal_integer_adapter_excludes_bool_and_fraction(self):
        validator = make_validator({"type": "integer"})
        for token, valid in [("1", True), ("1.0", True), ("1e0", True), ("-0", True),
                             ("0.5", False), ("true", False), ('"1"', False)]:
            self.assertEqual(validator.is_valid(schema_view(parse_raw(token))), valid, token)

    def test_schema_values_and_decoded_tempo_are_separate(self):
        for token in ["1e-400", "1e400", "2e-324"]:
            result = analyse(self.example.replace('"tempo": 120', '"tempo": ' + token), self.validator)
            self.assertTrue(result["schema_valid"])
            self.assertEqual(result["stage"], "numeric")
        for token in ["1e-300", "1e300", "3e-324", "1.7976931348623158e308"]:
            result = analyse(self.example.replace('"tempo": 120', '"tempo": ' + token), self.validator)
            self.assertEqual(result["status"], "valid")

    def test_integer_bounds_do_not_round_through_float(self):
        for token in ["9007199254740992", "9007199254740993", "18446744073709551616"]:
            result = analyse(self.example.replace('"start_tick": 0', '"start_tick": ' + token, 1), self.validator)
            self.assertEqual(result["stage"], "schema")
            self.assertFalse(result["schema_valid"])

    def test_note_end_is_independent_of_schema_field_bounds(self):
        raw = self.example.replace('"start_tick": 0', '"start_tick": 9007199254740991', 1)
        result = analyse(raw, self.validator)
        self.assertTrue(result["schema_valid"])
        self.assertEqual(result["stage"], "numeric")
        self.assertIn("derived note end", result["reason"])

    def test_hand_checked_bit_patterns(self):
        for token, bits in [("1", 0x3FF0000000000000), ("0.5", 0x3FE0000000000000),
                            ("0.1", 0x3FB999999999999A), ("120", 0x405E000000000000),
                            ("3e-324", 1), ("1.7976931348623158e308", MAX_BITS)]:
            self.assertEqual(binary64_bits(token), bits, token)
        for token in ["0", "-0", "-1", "1e-400", "1e400"]:
            self.assertIsNone(binary64_bits(token), token)

    def test_midpoint_ties_even_not_half_up(self):
        midpoint = "1.00000000000000011102230246251565404236316680908203125"
        self.assertEqual(binary64_bits(midpoint), 0x3FF0000000000000)
        self.assertEqual(round_even(5, 2), 2)
        self.assertEqual(round_even(7, 2), 4)

    def test_zero_and_infinity_thresholds(self):
        low = Fraction(1, 2**1075)
        self.assertIsNone(binary64_bits(decimal_text(low)))
        self.assertEqual(binary64_bits(decimal_text(low + Fraction(1, 2**1077))), 1)
        high = Fraction(2**1024 - 2**970)
        self.assertIsNone(binary64_bits(decimal_text(high)))
        self.assertEqual(binary64_bits(decimal_text(high - 1)), MAX_BITS)

    def test_decimal_context_does_not_change_oracle(self):
        token = decimal_text((bits_fraction(0x000FFFFFFFFFFFFF) + bits_fraction(0x0010000000000000)) / 2)
        with localcontext() as context:
            context.prec = 2
            self.assertEqual(binary64_bits(token), 0x0010000000000000)

    def test_remote_schema_retrieval_is_disabled(self):
        validator = make_validator({"$ref": "https://example.invalid/schema"})
        with self.assertRaises(Exception):
            list(validator.iter_errors(1))

    def test_fixed_corpus_expectations(self):
        for case in cases():
            with self.subTest(case=case["id"]):
                result = analyse(case["raw"], self.validator)
                self.assertEqual(result["schema_valid"], case["schema_valid"])
                if "valid" in case:
                    self.assertEqual(result["status"] == "valid", case["valid"])
                if case.get("bits"):
                    self.assertEqual(result.get("tempo_bits"), case["bits"])

    def test_sampler_is_reproducible_and_covers_all_finite_exponents(self):
        values = sampled_bits(10000, SEED)
        self.assertEqual(values, sampled_bits(10000, SEED))
        self.assertEqual(len(set(values)), 10000)
        self.assertTrue(all(0 < value <= MAX_BITS for value in values))
        self.assertEqual({value >> 52 for value in values}, set(range(2047)))
        self.assertEqual(sampled_bits(0, SEED), [])

    def test_comparison_catches_wrong_acceptance_bits_and_roundtrip(self):
        independent = analyse(self.example, self.validator)
        bits = independent["tempo_bits"]
        case = dict(score_valid=True, bits=bits)
        actual = dict(status="valid", bits=bits, serialized=self.example,
                      roundtrip=dict(status="valid", bits=bits))
        good = dict(from_str=actual, from_reader=copy.deepcopy(actual))
        self.assertEqual(compare(case, independent, good, self.validator), [])
        for mutation in [dict(status="invalid"), dict(bits="0000000000000001"),
                         dict(roundtrip=dict(status="valid", bits="0000000000000001"))]:
            wrong = copy.deepcopy(good)
            wrong["from_str"].update(mutation)
            self.assertTrue(compare(case, independent, wrong, self.validator))


if __name__ == "__main__":
    unittest.main()
