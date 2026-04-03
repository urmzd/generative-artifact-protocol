import argparse
import csv
import json
import re
import sys
from collections import Counter
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
        return data
    return None

def detect_anomalies(logs: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    times = [l['response_time'] for l in logs if 'response_time' in l and l['response_time'] > 0]
    if not times: return []
    
    threshold = sorted(times)[int(len(times) * 0.99)]
    return [l for l in logs if l.get('response_time', 0) > threshold]

def analyze_logs(logs: List[Dict[str, Any]], limit: int = 10) -> Dict[str, Any]:
    ips = [l['ip'] for l in logs if 'ip' in l]
    statuses = [l['status'] for l in logs if 'status' in l]
    times = [l['response_time'] for l in logs if 'response_time' in l]
    paths = [l['path'] for l in logs if 'path' in l]
    
    return {
        "top_ips": Counter(ips).most_common(limit),
        "status_distribution": dict(Counter(statuses)),
        "top_endpoints": Counter(paths).most_common(limit),
        "response_time_stats": {
            "p50": statistics.median(times) if times else 0,
            "p95": sorted(times)[int(len(times)*0.95)] if times else 0
        } if times else {},
        "anomalies": detect_anomalies(logs)
    }

def format_table(data: Dict[str, Any]) -> str:
    lines = ["--- Analysis Report ---"]
    for key, value in data.items():
        if key == "anomalies": continue
        lines.append(f"\n{key.replace('_', ' ').upper()}:")
        if isinstance(value, list):
            for item in value: lines.append(f"  {item[0]}: {item[1]}")
        elif isinstance(value, dict):
            for k, v in value.items(): lines.append(f"  {k}: {v}")
    
    lines.append("\nANOMALIES (P99+ Latency):")
    for a in data["anomalies"][:5]:
        lines.append(f"  Path: {a.get('path')} | Time: {a.get('response_time')}ms")
    return "\n".join(lines)

def main() -> None:
    parser = argparse.ArgumentParser(description="Log Analyzer Tool")
    parser.add_argument("file", help="Path to log file")
    parser.add_argument("--format", choices=["table", "json", "csv"], default="table")
    parser.add_argument("--limit", type=int, default=10)
    args = parser.parse_args()
    
    logs = []
    with open(args.file, 'r') as f:
        for line in f:
            parsed = parse_line(line)
            if parsed: logs.append(parsed)
                
    results = analyze_logs(logs, args.limit)
    
    if args.format == "json":
        print(json.dumps(results, indent=2))
    elif args.format == "csv":
        writer = csv.writer(sys.stdout)
        for k, v in results.items():
            if k != "anomalies": writer.writerow([k, str(v)])
    else:
        print(format_table(results))

if __name__ == "__main__":
    main()