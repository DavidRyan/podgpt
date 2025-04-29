{-# LANGUAGE OverloadedStrings #-}

module Gpt where

import Control.Monad (forM_)
import Control.Monad.IO.Class (liftIO)
import Data.Aeson (Value)
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import Network.OpenAI
import Network.OpenAI.Types

data Conversation = Conversation
  { _id :: Text,
    messages :: [Text]
  }
  deriving (Show)

data Gpt = Gpt
  { client :: OpenAIClient,
    conversation :: Conversation
  }

createGpt :: IO Gpt
createGpt = do
  let config = defaultOpenAIConfig
  client <- newOpenAIClient config
  let conversation = Conversation { _id = "1", messages = [] }
  return Gpt { client = client, conversation = conversation }

createImage :: Gpt -> Text -> IO (Either Text Text)
createImage gpt prompt = do
  let request =
        CreateImageRequest
          { prompt = prompt,
            responseFormat = Just Url,
            size = Just S256x256,
            user = Just "async-openai"
          }
  response <- runOpenAI (client gpt) $ createImage request
  case response of
    Left err -> return $ Left (T.pack $ show err)
    Right res -> do
      let urls = imageUrls res
      forM_ urls $ \url -> TIO.putStrLn $ "Image URL: " <> url
      return $ maybe (Left "No URLs returned") Right (listToMaybe urls)

replyToChat :: Gpt -> Text -> IO (Either Text Text)
replyToChat gpt prompt = do
  let updatedMessages = messages (conversation gpt) ++ [prompt]
  let mappedMessages =
        map
          (\msg -> ChatCompletionMessage { role = Just System, content = Just msg })
          updatedMessages
  let request =
        CreateChatCompletionRequest
          { model = "gpt-4o",
            messages = mappedMessages,
            maxTokens = Just 512
          }
  response <- runOpenAI (client gpt) $ createChatCompletion request
  case response of
    Left err -> return $ Left (T.pack $ show err)
    Right res -> do
      let msg = content =<< (head (choices res) >>= message)
      case msg of
        Nothing -> return $ Left "No response message"
        Just reply -> do
          let updatedConversation = (conversation gpt) { messages = updatedMessages ++ [reply] }
          return $ Right reply

createChat :: Gpt -> Text -> IO (Either Text Text)
createChat gpt prompt = do
  let request =
        CreateChatCompletionRequest
          { model = "gpt-4o",
            messages = [ChatCompletionMessage { role = Just System, content = Just prompt }],
            maxTokens = Just 512
          }
  response <- runOpenAI (client gpt) $ createChatCompletion request
  case response of
    Left err -> return $ Left (T.pack $ show err)
    Right res -> do
      let msg = content =<< (head (choices res) >>= message)
      case msg of
        Nothing -> return $ Left "No response message"
        Just reply -> do
          let newConversation = Conversation { _id = id res, messages = [prompt, reply] }
          return $ Right reply
