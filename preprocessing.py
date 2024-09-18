import os
import re
import json
import sys
from mitmproxy import io, tcp, http
import mitmproxy
from networkdata_pb2 import NetworkData,Header,Record,WebSocketMessage

#base_path = "./al/"
# base_path = "./archive/"
base_path = sys.argv[1]
os.chdir(base_path)
def main():
    for session_id in get_sessions("./"):
        mitmdump = "dump-"+session_id+".mitm"
        global_events = "events-"+session_id+".txt" # events-1726095578.txt
        app_uids = dict()
        app_connections = dict()
        app_network_data = dict()
        app_paths = set()
        missing_events = dict()
        device_ids = dict()
        # apps = set()
        with open(global_events, 'r') as f:
            for line in f:
                j = json.loads(line)
                first_key = next(iter(j))
                if first_key == "Other":
                    # print(j)
                    continue
                v = j[first_key]
                if "package_name" in v and "uid" in v and "ts" in v:
                    d = v["package_name"].split(":")[0]
                    # if first_key == "DeviceID":
                    #     device_id[d] = v["device_id"]
                    # print(device_id)
                    if v["uid"] not in app_uids:
                        app_uids[v["uid"]] = set()
                        app_uids[v["uid"]].add((d, v["ts"]))
                    else:
                        has_this_app = False
                        for r in app_uids[v["uid"]]:
                            if r[0] == d:
                                has_this_app = True
                                break
                        if not has_this_app:
                            app_uids[v["uid"]].add((d, v["ts"]))
        for d in [d for d in os.listdir() if not os.path.isfile(d)]:
            app_network_data[d] = NetworkData()
            app_network_data[d].pkg_name = d
            app_network_data[d].timestamp = int(session_id)
            # apps.add(d)
            app_paths.add(d)
        for d in app_paths:
            if not os.path.exists(d+"/analysis_result/events-"+session_id+".txt"):
                continue
            with open(d+"/analysis_result/events-"+session_id+".txt", 'r') as f:
                for line in f:
                    j = json.loads(line)
                    v = j[next(iter(j))]
                    if "uid" in v and "ts" in v:
                        if v["uid"] not in app_uids:
                            app_uids[v["uid"]] = set()
                            app_uids[v["uid"]].add((d, v["ts"]))
                            break
                        else:
                            has_this_app = False
                            for r in app_uids[v["uid"]]:
                                if r[0] == d:
                                    has_this_app = True
                                    break
                            if not has_this_app:
                                app_uids[v["uid"]].add((d, v["ts"]))
                            break
        
        # print(app_uids)
        unknown_events = set()
        with open(global_events, 'r') as f:
            for line in f:
                j = json.loads(line)
                first_key = next(iter(j))
                if first_key == "Other":
                    continue

                v = j[first_key]
                if "package_name" in v:
                    try:
                        d = v["package_name"].split(":")[0]
                    except:
                        print(v)
                        continue
                    if d not in missing_events:
                        missing_events[d] = set()
                        missing_events[d].add(line)
                    else:
                        missing_events[d].add(line)
                else:
                    unknown_events.add(line)
                    # print(v)
        for event in unknown_events:
            j = json.loads(event)
            first_key = next(iter(j))
            if first_key == "Other":
                continue

            v = j[first_key]
            ts = v["ts"]
            uid = v["uid"]
            if uid not in app_uids:
                continue
            app = find_closest_app(ts, app_uids[uid])
            if app not in missing_events:
                missing_events[app] = set()
                missing_events[app].add(event)
            else:
                missing_events[app].add(event)
        for app in missing_events:
            if not os.path.exists(app+"/analysis_result/"):
                os.makedirs(app+"/analysis_result/")
            app_events_missing = app+"/analysis_result/events-missing-"+session_id+".txt"
            try:
                os.remove(app_events_missing)
            except:
                pass
            with open(app_events_missing, 'w') as f:
                for line in missing_events[app]:
                    f.write(line)
        print("Missing events written")
        for d in app_paths:
            app_events = d+"/analysis_result/events-"+session_id+".txt"
            app_events_missing = d+"/analysis_result/events-missing-"+session_id+".txt"
            lines = set()
            if os.path.isfile(app_events):
                with open(app_events, 'r') as f:
                    for line in f:
                        lines.add(line)
            if os.path.isfile(app_events_missing):
                with open(app_events_missing, 'r') as f2:
                    for line in f2:
                        lines.add(line)
            for line in lines:
                addr = ""
                ts = ""
                j = json.loads(line)
                if "DeviceID" in j:
                    device_ids[d] = j["DeviceID"]["device_id"]
                if "SocketAddrV6" in j:
                    if "." in j["SocketAddrV6"]["left_socket_addr"]["ip"]:
                        addr = j["SocketAddrV6"]["left_socket_addr"]["ip"].split(":")[-1]
                    else:
                        addr = j["SocketAddrV6"]["left_socket_addr"]["ip"]
                    addr += ":"+str(j["SocketAddrV6"]["left_socket_addr"]["port"])
                    ts = j["SocketAddrV6"]["ts"]
                elif "SocketAddrV4" in j:
                    addr = j["SocketAddrV4"]["left_socket_addr"]["ip"].split(":")[-1]+":"+str(j["SocketAddrV4"]["left_socket_addr"]["port"])
                    ts = j["SocketAddrV4"]["ts"]
                elif "SocketHook" in j:
                    if len(j["SocketHook"]["local_addr"]) > 0:
                        addr = j["SocketHook"]["local_addr"]
                        ts = j["SocketHook"]["ts"]
                    else:
                        continue
                else:
                    continue
                if addr not in app_connections:
                    app_connections[addr] = set()    
                app_connections[addr].add((d, ts))
        # global_app_events = d+"/events-"+session_id+".txt"
        # for app in app_connections:
        #     for (d, ts) in app_connections[app]:
        #         pass
                # print(f"{app} -> {d} -> {ts}")
        # print(app_network_data)
                
        freader = io.FlowReader(open(mitmdump, "rb"))
        for request in freader.stream():
            if type(request) != mitmproxy.http.HTTPFlow:
                continue
            record = Record()
            record.request.path = request.request.path
            record.request.method = request.request.method
            request_headers,record.request.body,request_trailers = get_headers_and_body(request.request)
            record.request.headers.extend(request_headers)
            record.request.trailers.extend(request_trailers)
            record.connection_info.tls = request.client_conn.tls
            record.connection_info.timestamp = int(request.client_conn.timestamp_start*1000)
            if request.response is not None:
                record.response.status_code = request.response.status_code
                response_headers,record.response.body,response_trailers = get_headers_and_body(request.response)
                record.response.headers.extend(response_headers)
                record.response.trailers.extend(response_trailers)
            record.connection_info.source_address = request.client_conn.peername[0]+":"+str(request.client_conn.peername[1])
            try:
                record.connection_info.destination_address = request.server_conn.address[0]+":"+str(request.server_conn.address[1])
            except:
                pass
            if request.websocket is not None:
                for m in request.websocket.messages:
                    ws_message = WebSocketMessage()
                    ws_message.content = m.content
                    ws_message.from_client = m.from_client
                    record.websocket.extend([ws_message])        
            if record.connection_info.source_address in app_connections:
                # print(app_connections[record.connection_info.source_address])
                target_app = find_closest_app(record.connection_info.timestamp,app_connections[record.connection_info.source_address])
                app_network_data[target_app].records.extend([record])
        
        for pkg,data in app_network_data.items():
            app_events = pkg+"/analysis_result/network-"+session_id+".pb"  
            try:
                os.remove(app_events)
            except:
                pass
            with open(app_events, 'wb') as f:
                f.write(data.SerializeToString())
        for pkg,device_id in device_ids.items():
            app_id_file = pkg+"/analysis_result/device_id-"+session_id+".txt"
            try:
                os.remove(app_id_file)
            except:
                pass
            with open(app_id_file, 'w') as f:
                f.write(device_id)
            # print(y)
        # app_events = d+"/analysis_result/network-"+session_id+".pb"
        # with open(app_events, 'wb') as f:
        #     f.write(app_network_data[d].SerializeToString())
        continue

                    
def get_headers_and_body(r):
    headers = []
    trailers = []
    try:
        body = r.content if r.content is not None else b''
    except:
        r.decode(False)
        body = r.content if r.content is not None else b''

    for k,v in r.headers.items():
        try:
            header = Header()
            header.key = k
            header.value = v
            headers.append(header)
        except:
            pass
    if r.trailers is not None:
        for k,v in r.trailers.items():
            trailer = Header()
            trailer.key = k
            trailer.value = v
            trailers.append(trailer)
    return headers, body, trailers

def get_sessions(path: str):
    sessn = set()
    dir_files = os.listdir(path)
    for f in dir_files:
        if f.endswith('.mitm') and f.startswith('dump-') and os.path.isfile(path+f):
            if match := re.search('dump-(.*).mitm', f, re.IGNORECASE):
                sessn.add(match.group(1))
    return sessn

def find_closest_app(target, numbers):
    smallest_diff = None
    pkg = None
    
    for (p,num) in numbers:
        diff = abs(num - target)
        if smallest_diff is None or diff < smallest_diff:
            smallest_diff = diff
            pkg = p
    
    return pkg


if __name__ == "__main__":
    main()
