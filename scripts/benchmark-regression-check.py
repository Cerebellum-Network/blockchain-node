#!/usr/bin/env python3
"""
Runtime upgrade performance regression detection script.
Compares current benchmark results with baseline to detect performance regressions.
"""

import json
import sys
import argparse
from typing import Dict, List, Tuple, Optional

def load_benchmark_data(filepath: str) -> Dict:
    """Load benchmark data from JSON file."""
    try:
        with open(filepath, 'r') as f:
            return json.load(f)
    except FileNotFoundError:
        print(f"Error: Benchmark file {filepath} not found")
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON in {filepath}: {e}")
        sys.exit(1)

def compare_benchmarks(current: Dict, baseline: Dict, threshold: float = 0.1) -> Tuple[List[str], List[str]]:
    """
    Compare current benchmarks with baseline.
    
    Args:
        current: Current benchmark results
        baseline: Baseline benchmark results  
        threshold: Regression threshold (10% by default)
    
    Returns:
        Tuple of (regressions, improvements)
    """
    regressions = []
    improvements = []
    
    for pallet_name, pallet_data in current.items():
        if pallet_name not in baseline:
            print(f"Warning: New pallet {pallet_name} not in baseline")
            continue
            
        baseline_pallet = baseline[pallet_name]
        
        for extrinsic_name, current_times in pallet_data.items():
            if extrinsic_name not in baseline_pallet:
                print(f"Warning: New extrinsic {pallet_name}::{extrinsic_name} not in baseline")
                continue
                
            baseline_times = baseline_pallet[extrinsic_name]
            
            # Compare average execution times
            current_avg = sum(current_times) / len(current_times)
            baseline_avg = sum(baseline_times) / len(baseline_times)
            
            if baseline_avg == 0:
                continue
                
            change_ratio = (current_avg - baseline_avg) / baseline_avg
            
            if change_ratio > threshold:
                regressions.append(
                    f"{pallet_name}::{extrinsic_name}: "
                    f"{baseline_avg:.2f}ns → {current_avg:.2f}ns "
                    f"({change_ratio:.1%} regression)"
                )
            elif change_ratio < -threshold:
                improvements.append(
                    f"{pallet_name}::{extrinsic_name}: "
                    f"{baseline_avg:.2f}ns → {current_avg:.2f}ns "
                    f"({abs(change_ratio):.1%} improvement)"
                )
    
    return regressions, improvements

def check_critical_pallets(current: Dict, baseline: Dict) -> List[str]:
    """Check DDC-specific pallets for any regressions."""
    critical_pallets = [
        'pallet_ddc_clusters',
        'pallet_ddc_nodes', 
        'pallet_ddc_staking',
        'pallet_ddc_customers',
        'pallet_ddc_payouts'
    ]
    
    critical_regressions = []
    
    for pallet in critical_pallets:
        if pallet in current and pallet in baseline:
            # Even small regressions in critical pallets should be flagged
            regs, _ = compare_benchmarks(
                {pallet: current[pallet]}, 
                {pallet: baseline[pallet]}, 
                threshold=0.05  # 5% threshold for critical pallets
            )
            critical_regressions.extend(regs)
    
    return critical_regressions

def main():
    parser = argparse.ArgumentParser(description='Check for runtime upgrade performance regressions')
    parser.add_argument('current', help='Current benchmark results JSON file')
    parser.add_argument('baseline', help='Baseline benchmark results JSON file')
    parser.add_argument('--threshold', type=float, default=0.1, 
                       help='Regression threshold (default: 0.1 = 10%%)')
    parser.add_argument('--fail-on-regression', action='store_true',
                       help='Exit with error code if regressions found')
    
    args = parser.parse_args()
    
    current_data = load_benchmark_data(args.current)
    baseline_data = load_benchmark_data(args.baseline)
    
    print("🔍 Analyzing runtime upgrade performance impact...")
    print(f"Threshold: {args.threshold:.1%}")
    print()
    
    # General regression check
    regressions, improvements = compare_benchmarks(current_data, baseline_data, args.threshold)
    
    # Critical pallet specific check
    critical_regressions = check_critical_pallets(current_data, baseline_data)
    
    # Report results
    if regressions:
        print("❌ Performance Regressions Found:")
        for regression in regressions:
            print(f"  • {regression}")
        print()
    
    if critical_regressions:
        print("🚨 Critical DDC Pallet Regressions:")
        for regression in critical_regressions:
            print(f"  • {regression}")
        print()
    
    if improvements:
        print("✅ Performance Improvements:")
        for improvement in improvements:
            print(f"  • {improvement}")
        print()
    
    if not regressions and not critical_regressions:
        print("✅ No significant performance regressions detected!")
        print(f"📊 Found {len(improvements)} improvements")
    
    # Exit with error if regressions found and flag is set
    if args.fail_on_regression and (regressions or critical_regressions):
        print("\n💥 Failing due to performance regressions")
        sys.exit(1)
    
    sys.exit(0)

if __name__ == '__main__':
    main() 
