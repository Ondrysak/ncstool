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
            self.assertEqual(zf.read("projects/project_1.ncs"), self.funk)
            self.assertEqual(zf.read("projects/project_0.ncs"), self.deep)
            self.assertEqual(zf.read("samples/sample_0.wav"), sample_payload)
            rewritten_index = json.loads(zf.read("index.json"))

        self.assertEqual(rewritten_index["projects"][0], index["projects"][0])
        self.assertEqual(rewritten_index["projects"][1]["url"], "projects/project_1.ncs")
        self.assertEqual(rewritten_index["projects"][1]["name"], "Added Project")
        self.assertEqual(rewritten_index["samples"], index["samples"])

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
