import datetime as dt
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
from pathlib import Path

root = Path('/Volumes/SN850X/term4u-l0-20260925/round2')
log = root / 'logs' / 'cua08-http.jsonl'
class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        with log.open('a', encoding='utf-8') as out:
            out.write(json.dumps({'utc': dt.datetime.now(dt.timezone.utc).isoformat(),
                                  'method': 'GET', 'path': self.path}) + '\n')
        body = b'L0 loopback test fixture\n'
        self.send_response(200)
        self.send_header('Content-Type', 'text/plain; charset=utf-8')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def log_message(self, *_):
        pass
httpd = HTTPServer(('127.0.0.1', 0), Handler)
(root / 'fixtures' / 'http-port.txt').write_text(str(httpd.server_port), encoding='ascii')
print(json.dumps({'ready': True, 'address': '127.0.0.1', 'port': httpd.server_port}), flush=True)
httpd.serve_forever()
