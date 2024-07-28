#!/usr/bin/env python3
from dotenv import load_dotenv
import os
import logging
from pybit.unified_trading import HTTP
logging.basicConfig(level=logging.DEBUG)
load_dotenv()

API_KEY = os.getenv("BYBIT_API_KEY")
API_SECRET = os.getenv("BYBIT_API_SECRET")

session = HTTP(
    api_key=API_KEY,
    api_secret=API_SECRET,
)
session.log_requests = True

print(session.get_orderbook(category="linear", symbol="ETHUSDT"))

print(session.place_order(
    category="linear",
    symbol="ETHUSDT",
    side="Buy",
    orderType="Market",
    qty=0.01,
))
