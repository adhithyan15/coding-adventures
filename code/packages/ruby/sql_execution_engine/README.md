# sql_execution_engine (Ruby)

A **SELECT-only SQL execution engine** that executes parsed SQL queries against
any pluggable data source.

## Usage

```ruby
require "coding_adventures_sql_execution_engine"

class MySource
  include CodingAdventures::SqlExecutionEngine::DataSource

  def schema(table_name)
    case table_name
    when "users" then ["id", "name", "age"]
    else raise CodingAdventures::SqlExecutionEngine::TableNotFoundError.new(table_name)
    end
  end

  def scan(table_name)
    case table_name
    when "users"
      [{"id" => 1, "name" => "Alice", "age" => 30},
       {"id" => 2, "name" => "Bob",   "age" => 25}]
    else raise CodingAdventures::SqlExecutionEngine::TableNotFoundError.new(table_name)
    end
  end
end

source = MySource.new
result = CodingAdventures::SqlExecutionEngine.execute(
  "SELECT name FROM users WHERE age > 27",
  source
)
puts result.columns.inspect  # ["name"]
puts result.rows.inspect     # [{"name" => "Alice"}]
```
