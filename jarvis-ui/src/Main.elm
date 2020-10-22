module Main exposing (..)

import Browser
import Browser.Navigation as Nav
import Html exposing (Html, a, button, div, h1, li, p, text, ul)
import Html.Attributes exposing (class, href)
import Html.Events exposing (onClick)
import Http
import Router exposing (toRoute, Route)
import Url exposing (Protocol(..), toString)



-- MAIN


main : Program () Model Msg
main =
  Browser.application
    { init = init
    , view = view
    , update = update
    , subscriptions = subscriptions
    , onUrlChange = UrlChanged
    , onUrlRequest = LinkClicked
    }



-- MODEL


type alias Model =
  { key : Nav.Key
  , url : Url.Url
  , text : Maybe String
  }


init : () -> Url.Url -> Nav.Key -> ( Model, Cmd Msg )
init flags url key =
  ( Model key url Nothing, Cmd.none )



-- UPDATE


type Msg
  = GotText (Result Http.Error String)
  | LinkClicked Browser.UrlRequest
  | UrlChanged Url.Url
  | Refresh


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
  case msg of
    LinkClicked urlRequest ->
      case urlRequest of
        Browser.Internal url ->
          ( model, Nav.pushUrl model.key (Url.toString url) )

        Browser.External href ->
          ( model, Nav.load href )

    UrlChanged url ->
      ( { model | url = url }
      , Cmd.none
      )

    Refresh ->
      ( model
      , Http.get {
          url = "https://elm-lang.org/assets/public-opinion.txt"
          , expect = Http.expectString GotText
          }
      )

    GotText r ->
      case r of
        Ok fullText ->
          ({ model | text = Just fullText }
          , Cmd.none
          )

        Err _ ->
          (model, Cmd.none)



-- SUBSCRIPTIONS


subscriptions : Model -> Sub Msg
subscriptions _ =
  Sub.none



-- VIEW


view : Model -> Browser.Document Msg
view model =
  case (toRoute (toString model.url)) of
      Router.Home ->
          { title = "Jarvis"
          , body = [div [ class "fluid-container" ] [
              h1 [ class "bg-secondary pl-2 py-2" ] [ text "Welcome to Jarvis CI" ]
              , ul [] [
                viewLink "about" "/help/about"
              ]
              , button [ onClick Refresh ] [ text "Refresh" ]
              , div [] <|
                case model.text of
                    Nothing ->
                        [ p [] [ text "" ] ]
                    Just t ->
                        [ text t ]

           ]]
          }
      Router.Help topic ->
          { title = "Jarvis"
          , body = [
              div [ class "fluid-container" ]
              [
                h1 [ class "bg-secondary pl-2 py-2" ] [ text "Jarvis help" ]
                , p [] [ text ("Sorry, there's no help available for [" ++ topic ++ "]") ]
              ]
            ]
          }
      Router.NotFound ->
          { title = "Not found"
          , body = [ text "Page not found" ]
          }

viewLink : String -> String -> Html msg
viewLink title path =
  li [] [ a [ href path ] [ text title ] ]
