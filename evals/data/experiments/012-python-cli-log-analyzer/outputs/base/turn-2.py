import argparse
import csv
import json
import re
import sys
from collections import Counter, defaultdict
from datetime import datetime
from typing import List, Dict, Any, Optional
import statistics

APACHE_REGEX = r'(?P<ip>\S+) \S+ \S+ \[(?P<timestamp>.*?)\] "(?P<method>\S+) (?P<path>\S+) \S+" (?P<status>\d+) (?P<size>\S+)(?: (?P<response_time>\d+))?'

def parse_line(line: str) -> Optional[Dict[str, Any]]:
    line = line.strip()
    if not line: return None
    if line.startswith('{'):
        try: return json.loads(line)
        except json.JSONDecodeError: pass
    match = re.match(APACHE_REGEX, line)
    if match:
        data = match.groupdict()
        data['status'] = int(data['status'])
        data['response_time'] = int(data['response_time']) if data.get('response_time') else 0
        # Simple date parse for Apache: 10/Oct/2000:13:55:36
        ts_str = data['timestamp'].split(':')[0]
        try:
            data['dt'] = datetime.strptime(data['timestamp'].split(' ')[0], '%d/%b/%Y:%H:%M:%S')
        except:
            data['dt'] = datetime.now()
        return data
    return None

def group_by_time(logs: List[Dict[str, Any]], interval: str) -> Dict[str, int]:
    groups = defaultdict(int)
    for log in logs:
        dt = log.get('dt', datetime.now())
        if interval == 'hour': key = dt.strftime('%Y-%m-%d %H:00')
        elif interval == 'day': key = dt.strftime('%Y-%m-%d')
        elif interval == 'week': key = dt.strftime('%Y-W%U')
        groups[key] += 1
    return dict(groups)

def detect_anomalies(logs: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    times = [l['response_time'] for l in logs if 'response_time' in l and l['response_time'] > 0]
    if not times: return []
    threshold = sorted(times)[int(len(times) * 0.99)]
    return [l for l in logs if l.get('response_time', 0) > threshold]

def analyze_logs(logs: List[Dict[str, Any]], limit: int = 10, group_interval: Optional[str] = None) -> Dict[str, Any]:
    res = {
        "top_ips": Counter([l['ip'] for l in logs if 'ip' in l]).most_common(limit),
        "status_distribution": dict(Counter([l['status'] for l in logs if 'status' in l])),
        "anomalies": detect_anomalies(logs)
    }
    if group_interval:
        res["time_grouping"] = group_by_time(logs, group_interval)
    return res

def main() -> None:
    parser = argparse.ArgumentParser(description="Log Analyzer Tool")
    parser.add_argument("file", help="Path to log file")
    parser.add_argument("--format", choices=["table", "json", "csv"], default="table")
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--group-by", choices=["hour", "day", "week"], help="Time-based grouping")
    args = parser.parse_args()
    
    logs = [p for line in open(args.file) if (p := parse_line(line))]
    results = analyze_logs(logs, args.limit, args.group_by)
    
    if args.format == "json":
        print(json.dumps(results, indent=2, default=str))
    elif args.format == "csv":
        writer = csv.writer(sys.stdout)
        for k, v in results.items(): writer.writerow([k, str(v)])
    else:
        print("--- Analysis Report ---")
        for k, v in results.items():
            print(f"\n{k.replace('_', ' ').upper()}:")
            print(v)

if __name__ == "__main__":
    main()