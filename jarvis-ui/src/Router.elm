module Router exposing (Route(..), toRoute)

import Url
import Url.Parser exposing ((</>), Parser, int, map, oneOf, parse, s, string, top)

type Route
  = Home
  | Help String
  | NotFound

routeParser : Parser (Route -> a) a
routeParser =
  oneOf
    [ map Home   top
    , map Help    (s "help" </> string)
    ]

toRoute : String -> Route
toRoute string =
  case Url.fromString string of
    Nothing ->
      NotFound

    Just url ->
      Maybe.withDefault NotFound (parse routeParser url)
