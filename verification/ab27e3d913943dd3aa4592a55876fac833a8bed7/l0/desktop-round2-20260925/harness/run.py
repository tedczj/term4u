import datetime,json,os,subprocess,sys,time
from pathlib import Path
out=Path(__file__).parent
name=sys.argv[1]
command=sys.argv[2:]
start=datetime.datetime.now(datetime.timezone.utc).isoformat()
with (out/(name+'.log')).open('w') as log:
    try:
        result=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
    except OSError as error:
        log.write(str(error)+'\n')
        result=subprocess.CompletedProcess(command,127)
record=dict(command=command,exit_code=result.returncode,started_utc=start,finished_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
(out/(name+'.json')).write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps(record),flush=True)
sys.exit(result.returncode)
