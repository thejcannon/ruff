# Test cases for ASYNC900: Async generator without @asynccontextmanager not allowed

from contextlib import asynccontextmanager
import pytest


# Error: ASYNC900 - async generator without decorator
async def foo1():
    yield
    yield


# OK - has @asynccontextmanager decorator
@asynccontextmanager
async def foo2():
    yield


# Error: ASYNC900 - nested async generator without decorator  
@asynccontextmanager
async def foo3():
    async def bar():  # ASYNC900
        yield
    yield


# OK - regular sync function with yield
def foo4():
    yield


# OK - has @pytest.fixture decorator (should be treated as safe)
@pytest.fixture
async def async_fixtures_are_basically_context_managers():
    yield


# OK - has @pytest.fixture decorator with args
@pytest.fixture(scope="function")
async def async_fixtures_can_take_arguments():
    yield


# OK - not an async generator (no yield)
async def this_is_not_an_async_generator():
    @asynccontextmanager
    async def cm():
        yield

    async with cm():
        pass


# OK - nested function is not async
async def another_non_generator():
    def foo():
        yield


# Error: ASYNC900 - async generator without decorator
async def simple_async_gen():
    value = 0
    while True:
        yield value
        value += 1