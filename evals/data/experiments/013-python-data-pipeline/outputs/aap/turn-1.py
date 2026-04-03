import pandas as pd
import numpy as np
from dataclasses import dataclass
from typing import List
from pathlib import Path
import json

<aap:target id="pipeline-config">
@dataclass
class ETLConfig:
    input_path: str
    output_dir: str
    required_columns: List[str] = None
    date_col: str = "order_date"
</aap:target>

<aap:target id="pipeline-engine">
class SalesETLPipeline:
    def __init__(self, config: ETLConfig):
        self.config = config
        Path(config.output_dir).mkdir(parents=True, exist_ok=True)

    def extract(self, file_path: str) -> pd.DataFrame:
        <aap:target id="extraction-logic">
        df = pd.read_csv(file_path, encoding='utf-8-sig', parse_dates=[self.config.date_col])
        return df
        </aap:target>

    def validate(self, df: pd.DataFrame):
        <aap:target id="validation-logic">
        if df.isnull().any().any():
            raise ValueError("Null values detected in dataset")
        if df.duplicated().any():
            raise ValueError("Duplicate rows detected")
        </aap:target>

    def transform(self, df: pd.DataFrame) -> pd.DataFrame:
        <aap:target id="transformation-logic">
        # Clean column names
        df.columns = [c.lower().replace(' ', '_') for c in df.columns]
        
        # Derived metrics
        df['profit_margin'] = (df['revenue'] - df['cost']) / df['revenue']
        
        # Categorize
        df['product_tier'] = pd.cut(df['revenue'], bins=[0, 100, 500, np.inf], labels=['Low', 'Mid', 'High'])
        
        # Customer Lifetime Value
        clv = df.groupby('customer_id')['revenue'].sum().rename('customer_lifetime_value')
        df = df.merge(clv, on='customer_id', how='left')
        
        # Aggregate
        summary = df.groupby('region').agg({'revenue': 'sum', 'profit_margin': 'mean'})
        return df, summary
        </aap:target>

    def load(self, df: pd.DataFrame, summary: pd.DataFrame):
        <aap:target id="loading-logic">
        df.to_parquet(f"{self.config.output_dir}/processed_sales.parquet")
        summary.to_json(f"{self.config.output_dir}/regional_summary.json")
        </aap:target>

    def run(self, file_path: str):
        df = self.extract(file_path)
        self.validate(df)
        df, summary = self.transform(df)
        self.load(df, summary)
</aap:target>
