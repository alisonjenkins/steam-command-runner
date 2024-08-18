Feature: Config loading

  Scenario: We load a config with pre-command set
    Given the config file tests/features/config/config.toml
    When I load the config
    Then the config has pre-command set to "gamemoderun"
