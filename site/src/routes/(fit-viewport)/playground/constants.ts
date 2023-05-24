export const starting_querydown = `issues
created_at:>@6M|ago
--#assignments
++#labels{name:["Regression" "Bug"]}
#comments{user.team.name!"Backend"}:10
$author.username
$#comments.created_at%min \\sd`;