import pandas as pd
import numpy as np
import json
from dataclasses import dataclass
from typing import List
from pathlib import Path

@dataclass
class PipelineConfig:
    input_path: str
    output_dir: str
    date_cols: List[str]
    required_cols: List[str]

def extract(config: PipelineConfig) -> pd.DataFrame:
    df = pd.read_csv(
        config.input_path, 
        encoding='utf-8-sig', 
        parse_dates=config.date_cols
    )
    return df

def validate(df: pd.DataFrame, config: PipelineConfig):
    # Check required columns
    missing = [c for c in config.required_cols if c not in df.columns]
    if missing: raise ValueError(f"Missing columns: {missing}")
    
    # Null checks
    if df[config.required_cols].isnull().any().any():
        print("Warning: Nulls detected in critical columns")
        
    # Range check
    if (df['sales'] < 0).any():
        raise ValueError("Negative sales detected")
        
    # Duplicate check
    if df.duplicated().any():
        df.drop_duplicates(inplace=True)
        
    return df

def transform(df: pd.DataFrame) -> pd.DataFrame:
    # Clean column names
    df.columns = [c.lower().replace(' ', '_') for c in df.columns]
    
    # Derived metrics
    df['profit'] = df['revenue'] - df['cost']
    df['profit_margin'] = df['profit'] / df['revenue']
    
    # Categorization
    df['product_tier'] = pd.cut(
        df['revenue'], 
        bins=[0, 1000, 5000, np.inf], 
        labels=['Entry', 'Mid', 'Premium']
    )
    
    # YoY Growth (Requires sorted data)
    df = df.sort_values(['region', 'date'])
    df['yoy_growth'] = df.groupby('region')['revenue'].pct_change(periods=1)
    
    return df

def load(df: pd.DataFrame, output_dir: str):
    path = Path(output_dir)
    path.mkdir(parents=True, exist_ok=True)
    
    # Parquet
    df.to_parquet(path / "sales_processed.parquet")
    
    # JSON Summary
    summary = {
        "total_revenue": float(df['revenue'].sum()),
        "avg_margin": float(df['profit_margin'].mean()),
        "region_sales": df.groupby('region')['revenue'].sum().to_dict()
    }
    with open(path / "summary.json", 'w') as f:
        json.dump(summary, f, indent=4)

def run_pipeline(config: PipelineConfig):
    df = extract(config)
    df = validate(df, config)
    df = transform(df)
    load(df, config.output_dir)
    print("Pipeline completed successfully.")

if __name__ == "__main__":
    config = PipelineConfig(
        input_path="sales_data.csv",
        output_dir="./output",
        date_cols=["date"],
        required_cols=["date", "region", "revenue", "cost"]
    )
    # run_pipeline(config)