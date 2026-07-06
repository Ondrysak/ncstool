import json
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT = REPO_ROOT / "scripts" / "pack_repack.py"
DEEP_NCS = REPO_ROOT / "test_data" / "Deep.ncs"
FUNK_NCS = REPO_ROOT / "test_data" / "Funk.ncs"

FILE_SIZE = 160_780
VELOCITY_OFF = 0x0CD74
PROBABILITY_OFF = 0x0CD94
TRACK_STRIDE = 0x3540
PATTERN_STRIDE = 0x06A8
DRUM_CHOICE_OFF = 0x0CDB4
DRUM_RHYTHM_OFF = 0x0CDD4
DEFAULT_DRUM_CHOICES_OFF = 0x1A278
SYNTH_TRACK_INFO_OFF = 0x0CD64
TRACK_INFO_STRIDE = 8
STEPS = 32
PROJECT_NAME_LEN_OFF = 0x0C
PROJECT_NAME_OFF = 0x10
PROJECT_NAME_BYTES = 32
PROJECT_NAME_FIELD_END = PROJECT_NAME_OFF + PROJECT_NAME_BYTES



def project_offset(track, pattern, step):
    return track * TRACK_STRIDE + pattern * PATTERN_STRIDE + step


class PackRepackCommandTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.work = Path(self.tmp.name)
        self.deep = DEEP_NCS.read_bytes()
        self.funk = FUNK_NCS.read_bytes()
        self.assertEqual(len(self.deep), FILE_SIZE)
        self.assertEqual(len(self.funk), FILE_SIZE)

    def write_pack(self, path, index, entries):
        with zipfile.ZipFile(path, "w") as zf:
            zf.writestr("index.json", json.dumps(index, indent=2).encode("utf-8"))
            for name, payload in entries.items():
                zf.writestr(name, payload)

    def run_repacker(self, *args):
        return subprocess.run(
            [sys.executable, str(SCRIPT), *map(str, args)],
            cwd=REPO_ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )

    def write_generation_pack(self, path, index, project_entries=None):
        entries = dict(project_entries or {"projects/project_0.ncs": self.deep})
        for section in ("samples", "patches"):
            for item in index.get(section, []):
                entries[item["url"]] = f"{section}:{item['name']}".encode("utf-8")
        self.write_pack(path, index, entries)

    def project_step_bytes(self, project, base_offset, track, pattern=0):
        return [
            project[base_offset + project_offset(track, pattern, step)]
            for step in range(STEPS)
        ]

    def assert_project_display_name(self, project, name):
        raw = name.encode("ascii")
        self.assertLessEqual(len(raw), PROJECT_NAME_BYTES)
        self.assertEqual(
            project[PROJECT_NAME_LEN_OFF : PROJECT_NAME_LEN_OFF + 4],
            len(raw).to_bytes(4, "little"),
        )
        self.assertEqual(
            project[PROJECT_NAME_OFF:PROJECT_NAME_FIELD_END],
            raw + b" " * (PROJECT_NAME_BYTES - len(raw)),
        )

    def assert_project_bytes_match_except_display_name(self, project, expected):
        self.assertEqual(project[:PROJECT_NAME_LEN_OFF], expected[:PROJECT_NAME_LEN_OFF])
        self.assertEqual(project[PROJECT_NAME_FIELD_END:], expected[PROJECT_NAME_FIELD_END:])


    def test_edit_updates_drum_step_velocity_and_probability_bytes_without_touching_other_entries(self):
        src = self.work / "source.circuittrackspack"
        dst = self.work / "edited.circuittrackspack"
        index = {
            "projects": [
                {"name": "Editable", "url": "projects/project_0.ncs"},
                {"name": "Preserved", "url": "projects/project_1.ncs"},
            ],
            "samples": [{"name": "Kick", "url": "samples/sample_0.wav"}],
        }
        sample_payload = b"sample payload must survive byte-for-byte"
        self.write_pack(
            src,
            index,
            {
                "projects/project_0.ncs": self.deep,
                "projects/project_1.ncs": self.funk,
                "samples/sample_0.wav": sample_payload,
            },
        )

        result = self.run_repacker("edit", src, dst, 0, "2:3:Xx.9:6")

        self.assertEqual(result.returncode, 0, result.stderr)
        with zipfile.ZipFile(src, "r") as original, zipfile.ZipFile(dst, "r") as edited:
            self.assertEqual(edited.namelist(), original.namelist())
            self.assertEqual(edited.read("index.json"), original.read("index.json"))
            self.assertEqual(edited.read("projects/project_1.ncs"), self.funk)
            self.assertEqual(edited.read("samples/sample_0.wav"), sample_payload)

            edited_project = edited.read("projects/project_0.ncs")
            original_project = original.read("projects/project_0.ncs")

        self.assertEqual(len(edited_project), FILE_SIZE)
        expected_target_bytes = {}
        for step, expected_velocity, expected_probability in [
            (0, 127, 6),
            (1, 32, 6),
            (2, 0, 0),
            (3, 127, 6),
        ]:
            offset = project_offset(track=2, pattern=3, step=step)
            velocity_pos = VELOCITY_OFF + offset
            probability_pos = PROBABILITY_OFF + offset
            expected_target_bytes[velocity_pos] = expected_velocity
            expected_target_bytes[probability_pos] = expected_probability
            self.assertEqual(edited_project[velocity_pos], expected_velocity)
            self.assertEqual(edited_project[probability_pos], expected_probability)

        actual_changes = {
            pos
            for pos, (before, after) in enumerate(zip(original_project, edited_project))
            if before != after
        }
        self.assertEqual(
            actual_changes,
            {
                pos
                for pos, expected in expected_target_bytes.items()
                if original_project[pos] != expected
            },
        )

    def test_replace_adds_missing_project_entry_and_updates_index_name(self):
        src = self.work / "source.circuittrackspack"
        dst = self.work / "expanded.circuittrackspack"
        index = {
            "projects": [
                {"name": "Existing", "url": "projects/project_0.ncs"},
                {"name": "Empty slot", "url": "projects/project_1.ncs"},
            ],
            "samples": [{"name": "Hat", "url": "samples/sample_0.wav"}],
        }
        sample_payload = b"unchanged sample"
        self.write_pack(
            src,
            index,
            {
                "projects/project_0.ncs": self.deep,
                "samples/sample_0.wav": sample_payload,
            },
        )

        result = self.run_repacker("replace", src, dst, 1, FUNK_NCS, "--name", "Added Project")

        self.assertEqual(result.returncode, 0, result.stderr)
        with zipfile.ZipFile(dst, "r") as zf:
            self.assertIn("projects/project_1.ncs", zf.namelist())
            replaced_project = zf.read("projects/project_1.ncs")
            self.assertEqual(zf.read("projects/project_0.ncs"), self.deep)
            self.assertEqual(zf.read("samples/sample_0.wav"), sample_payload)
            rewritten_index = json.loads(zf.read("index.json"))

        self.assertEqual(rewritten_index["projects"][0], index["projects"][0])
        self.assertEqual(rewritten_index["projects"][1]["url"], "projects/project_1.ncs")
        self.assertEqual(rewritten_index["projects"][1]["name"], "Added Project")
        self.assertEqual(rewritten_index["samples"], index["samples"])
        self.assert_project_display_name(
            replaced_project,
            rewritten_index["projects"][1]["name"],
        )
        self.assert_project_bytes_match_except_display_name(replaced_project, self.funk)

    def test_generate_adds_missing_project_entry_updates_index_name_and_writes_project_bytes(self):
        src = self.work / "source.circuittrackspack"
        dst = self.work / "generated.circuittrackspack"
        index = {
            "projects": [
                {"name": "Template", "url": "projects/project_0.ncs"},
                {"name": "Empty slot", "url": "projects/project_1.ncs"},
            ],
            "samples": [
                {"name": "Tight Kick", "url": "samples/kick.wav"},
                {"name": "Real Snare", "url": "samples/snare.wav"},
                {"name": "Closed Hat", "url": "samples/hat.wav"},
                {"name": "Open Hat", "url": "samples/open_hat.wav"},
            ],
            "patches": [
                {"name": "Sub Bass", "url": "patches/sub.syx"},
                {"name": "Dream Pad", "url": "patches/pad.syx"},
            ],
        }
        self.write_generation_pack(src, index)

        result = self.run_repacker(
            "generate",
            src,
            dst,
            1,
            "--template",
            0,
            "--name",
            "Generated Slot",
            "--style",
            "techno",
            "--seed",
            99,
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        with zipfile.ZipFile(src, "r") as original, zipfile.ZipFile(dst, "r") as generated:
            self.assertNotIn("projects/project_1.ncs", original.namelist())
            self.assertIn("projects/project_1.ncs", generated.namelist())
            self.assertEqual(generated.read("projects/project_0.ncs"), self.deep)
            self.assertEqual(generated.read("samples/kick.wav"), b"samples:Tight Kick")
            generated_project = generated.read("projects/project_1.ncs")
            rewritten_index = json.loads(generated.read("index.json"))

        self.assertEqual(len(generated_project), FILE_SIZE)
        self.assertNotEqual(generated_project, self.deep)
        self.assertEqual(rewritten_index["projects"][0], index["projects"][0])
        self.assertEqual(rewritten_index["projects"][1]["url"], "projects/project_1.ncs")
        self.assertEqual(rewritten_index["projects"][1]["name"], "Generated Slot")
        self.assert_project_display_name(
            generated_project,
            rewritten_index["projects"][1]["name"],
        )
        self.assertEqual(
            generated_project[DEFAULT_DRUM_CHOICES_OFF : DEFAULT_DRUM_CHOICES_OFF + 4],
            bytes([0, 1, 2, 3]),
        )
        self.assertEqual(generated_project[SYNTH_TRACK_INFO_OFF], 0)
        self.assertEqual(generated_project[SYNTH_TRACK_INFO_OFF + TRACK_INFO_STRIDE], 1)

    def test_generate_writes_categorized_choices_synth_patches_and_drum_pattern_bytes(self):
        src = self.work / "source.circuittrackspack"
        dst = self.work / "generated.circuittrackspack"
        index = {
            "projects": [{"name": "Template", "url": "projects/project_0.ncs"}],
            "samples": [
                {"name": "Texture", "url": "samples/texture.wav"},
                {"name": "Tight Kick", "url": "samples/kick.wav"},
                {"name": "Real Snare", "url": "samples/snare.wav"},
                {"name": "Closed Hat", "url": "samples/hat.wav"},
                {"name": "Open Hat", "url": "samples/open_hat.wav"},
                {"name": "Ride Cymbal", "url": "samples/ride.wav"},
            ],
            "patches": [
                {"name": "Initial Patch", "url": "patches/initial.syx"},
                {"name": "Sub Bass", "url": "patches/sub.syx"},
                {"name": "Dream Pad", "url": "patches/pad.syx"},
                {"name": "Acid Lead", "url": "patches/lead.syx"},
            ],
        }
        self.write_generation_pack(src, index)

        result = self.run_repacker(
            "generate",
            src,
            dst,
            0,
            "--style",
            "techno",
            "--seed",
            1234,
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        with zipfile.ZipFile(dst, "r") as zf:
            generated_project = zf.read("projects/project_0.ncs")

        self.assertEqual(
            generated_project[DEFAULT_DRUM_CHOICES_OFF : DEFAULT_DRUM_CHOICES_OFF + 4],
            bytes([1, 2, 3, 4]),
        )
        self.assertEqual(generated_project[SYNTH_TRACK_INFO_OFF], 1)
        self.assertEqual(generated_project[SYNTH_TRACK_INFO_OFF + TRACK_INFO_STRIDE], 2)

        expected_hits_by_track = {
            0: {0: 118, 8: 118, 16: 118, 24: 118},
            1: {8: 112, 24: 112},
            2: {step: (96 if step % 4 == 0 else 72) for step in range(0, STEPS, 2)},
            3: {4: 90, 12: 90, 20: 90, 28: 90},
        }
        for track, expected_hits in expected_hits_by_track.items():
            self.assertEqual(
                self.project_step_bytes(generated_project, VELOCITY_OFF, track),
                [expected_hits.get(step, 0) for step in range(STEPS)],
            )
            self.assertEqual(
                self.project_step_bytes(generated_project, PROBABILITY_OFF, track),
                [7 if step in expected_hits else 0 for step in range(STEPS)],
            )
            self.assertEqual(
                self.project_step_bytes(generated_project, DRUM_RHYTHM_OFF, track),
                [1 if step in expected_hits else 0 for step in range(STEPS)],
            )
            self.assertEqual(
                self.project_step_bytes(generated_project, DRUM_CHOICE_OFF, track),
                [255] * STEPS,
            )

    def test_generate_seed_is_deterministic_for_selection_and_project_output(self):
        src = self.work / "source.circuittrackspack"
        first = self.work / "first.circuittrackspack"
        second = self.work / "second.circuittrackspack"
        index = {
            "projects": [
                {"name": "Template", "url": "projects/project_0.ncs"},
                {"name": "Empty slot", "url": "projects/project_1.ncs"},
            ],
            "samples": [
                {"name": "Tight Kick A", "url": "samples/kick_a.wav"},
                {"name": "Soft Kick B", "url": "samples/kick_b.wav"},
                {"name": "Real Snare A", "url": "samples/snare_a.wav"},
                {"name": "Clap Snare B", "url": "samples/snare_b.wav"},
                {"name": "Closed Hat A", "url": "samples/hat_a.wav"},
                {"name": "Bright Hat B", "url": "samples/hat_b.wav"},
                {"name": "Open Hat A", "url": "samples/open_hat.wav"},
                {"name": "Perc Ride B", "url": "samples/perc.wav"},
            ],
            "patches": [
                {"name": "Sub Bass A", "url": "patches/sub_a.syx"},
                {"name": "Acid Bass B", "url": "patches/acid_b.syx"},
                {"name": "Warm Pad A", "url": "patches/pad_a.syx"},
                {"name": "Bell Pad B", "url": "patches/pad_b.syx"},
                {"name": "Lead Tone C", "url": "patches/lead_c.syx"},
            ],
        }
        self.write_generation_pack(src, index)

        first_result = self.run_repacker(
            "generate",
            src,
            first,
            1,
            "--name",
            "Seeded",
            "--style",
            "house",
            "--seed",
            2026,
        )
        second_result = self.run_repacker(
            "generate",
            src,
            second,
            1,
            "--name",
            "Seeded",
            "--style",
            "house",
            "--seed",
            2026,
        )

        self.assertEqual(first_result.returncode, 0, first_result.stderr)
        self.assertEqual(second_result.returncode, 0, second_result.stderr)
        self.assertEqual(
            [
                line
                for line in first_result.stdout.splitlines()
                if line.startswith("  drums:") or line.startswith("  synths:")
            ],
            [
                line
                for line in second_result.stdout.splitlines()
                if line.startswith("  drums:") or line.startswith("  synths:")
            ],
        )
        with zipfile.ZipFile(first, "r") as first_pack, zipfile.ZipFile(second, "r") as second_pack:
            self.assertEqual(
                first_pack.read("projects/project_1.ncs"),
                second_pack.read("projects/project_1.ncs"),
            )
            self.assertEqual(first_pack.read("index.json"), second_pack.read("index.json"))

    def test_invalid_edit_spec_fails_without_creating_destination_pack(self):
        src = self.work / "source.circuittrackspack"
        dst = self.work / "should_not_exist.circuittrackspack"
        index = {"projects": [{"name": "Editable", "url": "projects/project_0.ncs"}]}
        self.write_pack(src, index, {"projects/project_0.ncs": self.deep})

        result = self.run_repacker("edit", src, dst, 0, "0:0:X?X")

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("invalid step char", result.stderr)
        self.assertFalse(dst.exists())


if __name__ == "__main__":
    unittest.main()
