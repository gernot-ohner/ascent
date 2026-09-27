"""The benchmark timeout must also stop the measured child of /usr/bin/time."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

spec = importlib.util.spec_from_file_location("openjdk_runner", Path(__file__).resolve().parents[1] / "benchmarks/openjdk.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class ProcessTests(unittest.TestCase):
    def test_nonzero_exit_is_preserved(self):
        result = runner.run_process([sys.executable, "-c", "import sys; print('failed',file=sys.stderr); sys.exit(7)"], 5)
        self.assertEqual(result.returncode, 7)
        self.assertIn("failed", result.stderr)

    def test_timeout_stops_descendants(self):
        with tempfile.TemporaryDirectory() as folder:
            marker = Path(folder) / "child-survived"
            ready = Path(folder) / "child-started"
            child = f"import time; from pathlib import Path; Path({str(ready)!r}).touch(); time.sleep(1); Path({str(marker)!r}).touch()"
            parent = f"import subprocess,sys,time; subprocess.Popen([sys.executable,'-c',{child!r}]); time.sleep(10)"
            with self.assertRaises(subprocess.TimeoutExpired):
                runner.run_process([sys.executable, "-c", parent], 0.5)
            self.assertTrue(ready.exists(), "child must have started before timeout")
            time.sleep(1)
            self.assertFalse(marker.exists(), "timed-out benchmark child is still running")


if __name__ == "__main__":
    unittest.main()
