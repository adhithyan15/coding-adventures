-- Conduit — a Sinatra/Express-style web framework backed by a Rust engine.
--
-- This module re-exports everything a typical Conduit application needs.
-- Import just `Conduit` and you get the full DSL.
--
-- QUICK START
-- -----------
--   import Conduit
--
--   main :: IO ()
--   main = do
--     app <- newApplication
--     get app "/" $ \_ ->
--       return (html 200 "<h1>Hello from Haskell!</h1>")
--     srv <- bind app "127.0.0.1" 8080
--     serve srv

module Conduit
  ( -- * Application
    Application
  , newApplication
  , addRoute
  , get, post, put, delete, patch, options
  , before
  , after
  , notFound
  , onError
  , setSetting
  , getSetting
  , bind

    -- * Server
  , Server
  , serve
  , serveBackground
  , stop
  , localPort
  , running
  , freeServer

    -- * Request
  , Request (..)
  , reqParam
  , reqQuery
  , reqHeader

    -- * Response
  , Response (..)
  , ConduitHalt (..)
  , respond
  , html
  , json
  , textPlain
  , redirect
  , halt
  ) where

import Conduit.App
import Conduit.Server
import Conduit.Request
import Conduit.Response
