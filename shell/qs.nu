def --env qs [...args] {
    let result = (^qs ...$args)
    if ($result | is-not-empty) {
        cd $result
    }
}
