"""Create an isolated 100,000-commit fixture and benchmark the release Git engine."""
import subprocess, tempfile, pathlib, sys, platform
count=int(sys.argv[1]) if len(sys.argv)>1 else 100_000
with tempfile.TemporaryDirectory(prefix='git-toys-benchmark-') as directory:
    root=pathlib.Path(directory)
    subprocess.run(['git','init','-b','main',directory],check=True,stdout=subprocess.DEVNULL)
    process=subprocess.Popen(['git','fast-import','--quiet'],cwd=root,stdin=subprocess.PIPE)
    for index in range(count):
        message=f'Synthetic benchmark commit {index}'
        data=f'commit refs/heads/main\ncommitter Benchmark <benchmark@example.invalid> {1700000000+index} +0000\ndata {len(message)}\n{message}\n\n'
        process.stdin.write(data.encode())
    process.stdin.close()
    if process.wait()!=0: raise SystemExit('Fixture generation failed')
    print(f'Fixture: {count} commits; {platform.platform()}; {platform.machine()}',flush=True)
    subprocess.run(['cargo','run','--release','--locked','-p','git-toys-engine','--example','benchmark','--',directory],check=True)
