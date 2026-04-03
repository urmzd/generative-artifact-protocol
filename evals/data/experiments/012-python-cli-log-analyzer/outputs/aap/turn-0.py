<aap:target id="log-analyzer-script">import argparse
import json
import re
import csv
from datetime import datetime
from collections import Counter, defaultdict
from typing import List, Dict, Any, Optional

<aap:target id="log-parser">def parse_log_line(line: str) -> Optional[Dict[str, Any]]:
    # Simple regex for Common/Nginx log format
    regex = r'(?P<ip>\S+) \S+ \S+ \[(?P<timestamp>.*?)\] "(?P<method>\S+) (?P<url>\S+) \S+" (?P<status>\d+) (?P<size>\d+) (?P<response_time>\d+\.\d+)'
    match = re.match(regex, line)
    if match:
        return match.groupdict()
    try:
        return json.loads(line)
    except:
        return None
</aap:target>

<aap:target id="analysis-functions">def analyze_logs(logs: List[Dict[str, Any]], top_n: int = 10) -> Dict[str, Any]:
    ips = Counter(log['ip'] for log in logs)
    status_codes = Counter(log['status'] for log in logs)
    endpoints = Counter(log['url'] for log in logs)
    response_times = [float(log['response_time']) for log in logs if 'response_time' in log]
    
    return {
        "top_ips": ips.most_common(top_n),
        "status_distribution": dict(status_codes),
        "top_endpoints": endpoints.most_common(top_n),
        "avg_response_time": sum(response_times) / len(response_times) if response_times else 0
    }
</aap:target>

<aap:target id="formatters">def format_output(data: Dict[str, Any], fmt: str) -> str:
    if fmt == 'json':
        return json.dumps(data, indent=2)
    elif fmt == 'csv':
        output = []
        for key, value in data.items():
            output.append(f"{key},{value}")
        return "\n".join(output)
    else:
        # Simple ASCII table
        lines = ["Analysis Report", "---------------"]
        for k, v in data.items():
            lines.append(f"{k.replace('_', ' ').title()}: {v}")
        return "\n".join(lines)
</aap:target>

<aap:target id="main-function">def main() -> None:
    parser = argparse.ArgumentParser(description="CLI Log Analyzer")
    parser.add_argument("--file", required=True, help="Path to log file")
    parser.add_argument("--format", choices=['table', 'json', 'csv'], default='table')
    parser.add_argument("--top", type=int, default=10)
    
    args = parser.parse_args()
    
    with open(args.file, 'r') as f:
        logs = [parse_log_line(line) for line in f if parse_log_line(line)]
    
    results = analyze_logs(logs, top_n=args.top)
    print(format_output(results, args.format))

if __name__ == "__main__":
    main()</aap:target>
</aap:target>