#!/usr/bin/env python

import os
import sys
import json
import logging
import tempfile
import functools

print = functools.partial(print, flush=True)

script_dir = os.path.dirname(os.path.realpath(__file__))
log_dir = os.path.join(script_dir, 'logs')
tmp_dir = tempfile.gettempdir()

# create log dir if not exist
if not os.path.exists(log_dir):
    os.makedirs(log_dir)

logging.basicConfig(filename=os.path.join(log_dir,'git-lfs-py.error.log'),level=logging.DEBUG)

def respond(obj):
    print(json.dumps(obj))
    # logging.debug('response: ' + json.dumps(obj))

def main():
    # print all args
    # logging.debug('args: ' + str(sys.argv))
    scp_args = sys.argv[1:]
    scp_arg_str = ' '.join(scp_args)
    
    for line in sys.stdin:
        # logging.debug('line: ' + line)
        obj = json.loads(line)
        if obj["event"] == "init":
            respond({})
        elif obj["event"] == "terminate":
            pass
        elif obj["event"] == "upload":
            # example obj
            # { "event": "upload", "oid": "bf3e3e2af9366a3b704ae0c31de5afa64193ebabffde2091936ad2e7510bc03a", "size": 346232, "path": "/path/to/file.png", "action": { "href": "nfs://server/path", "header": { "key": "value" } } }

            # response example
            # { "event": "complete", "oid": "bf3e3e2af9366a3b704ae0c31de5afa64193ebabffde2091936ad2e7510bc03a" }

            # call scp to upload file
            src_path = obj["path"]
            dst_path = os.path.join(scp_arg_str, obj["oid"])
            cmd = f"scp {src_path} {dst_path}"
            # logging.debug('cmd: ' + cmd)

            # get flag, if failed return error
            flag = os.system(cmd)
            if flag == 0:
                respond({ "event": "complete", "oid":obj["oid"]})
            else:
                respond({ "event": "complete", "oid":obj["oid"], "error": {"code":1, "message": "Upload failed"}})

                # error log
                logging.error('upload failed: ' + cmd + "( request json: " + json.dumps(obj) + ")")


        elif obj["event"] == "download":
            # example obj
            # {"event":"download","oid":"3637d3e5dc180a7f9b3ba04c7dd0cc5686ca76d57119ece79857ddb8e61c717b","size":129969,"action":null}

            # response example
            # { "event": "complete", "oid": "22ab5f63670800cc7be06dbed816012b0dc411e774754c7579467d2536a9cf3e", "path": "/path/to/file.png" }

            # call scp to download file
            tmp_file = tempfile.NamedTemporaryFile(dir=tmp_dir, delete=False)

            src_path = os.path.join(scp_arg_str, obj["oid"])
            cmd = f"scp {src_path} {tmp_file.name}"
            # logging.debug('cmd: ' + cmd)

            # get flag, if failed return error
            flag = os.system(cmd)
            if flag == 0:
                respond({ "event": "complete", "oid":obj["oid"], "path": tmp_file.name})
            else:
                respond({ "event": "complete", "oid":obj["oid"], "error": {"code":1, "message": "Download failed"}})

                # error log
                logging.error('download failed: ' + cmd + "( request json: " + json.dumps(obj) + ")")

if __name__ == "__main__":
    main()